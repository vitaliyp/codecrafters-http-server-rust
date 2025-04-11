use crate::concurrency::ThreadPool;
use crate::http;
use crate::http::handler::HandlerFunc;
use crate::http::method::Method;
use crate::http::middleware::{Middleware, Next};
use crate::http::middleware::compression::CompressionMw;
use crate::http::request::{Request, RequestContext};
use crate::http::{Response, BUFFER_SIZE};
use anyhow::{anyhow, bail, Context};
use once_cell::sync::Lazy;
use regex::{Captures, Regex};
use std::cmp::min;
use std::collections::HashMap;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::str::FromStr;
use std::sync::Arc;
use std::time::Duration;

pub struct Handler {
    method: Method,
    regex: Regex,
    pub f: HandlerFunc,
}

pub struct Server {
    listener: TcpListener,
    handlers: Vec<Handler>,
    pool: ThreadPool,
    middlewares: Vec<Box<dyn Middleware>>,
}

impl Server {
    fn new(listener: TcpListener, num_workers: usize) -> Server {
        let mut s = Server {
            listener,
            handlers: Vec::new(),
            pool: ThreadPool::new(num_workers),
            middlewares: Vec::new(),
        };

        s.add_middleware(Box::new(CompressionMw {}));
        s
    }

    pub fn from_tcp_addr(addr: &str, num_workers: usize) -> Result<Server, &'static str> {
        let listener = TcpListener::bind(addr).map_err(|_| "Can't bind address")?;
        Ok(Server::new(listener, num_workers))
    }

    const PATTERN_RE: Lazy<Regex> = Lazy::new(|| Regex::new(r"<(?P<var>[a-z][a-z0-9]*)>").unwrap());

    pub fn add_handler(&mut self, m: Method, pattern: &str, f: HandlerFunc) {
        let pattern = Self::PATTERN_RE
            .replace_all(pattern, |capt: &Captures| {
                format!(r"(?<{}>[^/?]+)", capt.name("var").unwrap().as_str())
            })
            .to_string();
        let pattern = format!("^{}$", pattern);

        let compiled = Regex::new(&pattern).unwrap();

        self.handlers.push(Handler {
            method: m,
            regex: compiled,
            f,
        })
    }

    pub fn add_middleware(&mut self, m: Box<dyn Middleware>) {
        self.middlewares.push(m);
    }

    pub fn run(self) -> Result<(), &'static str> {
        let server = Arc::new(self);
        for stream in server.listener.incoming() {
            let stream = stream.map_err(|_| "Error listening")?;
            let thread_server = Arc::clone(&server);
            server
                .pool
                .execute(move || thread_server.process_incoming(stream));
        }
        Ok(())
    }

    fn process_incoming(&self, mut stream: TcpStream) {
        stream
            .set_read_timeout(Some(Duration::from_secs(5)))
            .unwrap();

        println!("accepted new connection: {:?}", stream.peer_addr());

        let request = Self::read_request(&mut stream);

        let response = if let Ok(request) = request {
            self.dispatch(request)
        } else {
            http::bad_request()
        };

        stream
            .write_all(serialize_response(&response).as_ref())
            .unwrap();
    }

    fn dispatch(&self, req: Request) -> Response {
        let handler = self
            .handlers
            .iter()
            .map(|h| (h, h.regex.captures(&req.url)))
            .find(|(h, c)| h.method == req.method && c.is_some());

        if let Some((handler, Some(capt))) = handler {
            let vars = Self::get_url_vars(handler, capt);
            let mut req_ctx = RequestContext::from(req, vars);

            let next = Next {
                middlewares: self.middlewares.as_ref(),
                handler: &handler.f,
            };

            next.run(&mut req_ctx)
        } else {
            http::not_found()
        }
    }

    fn get_url_vars(handler: &Handler, capt: Captures) -> HashMap<String, String> {
        handler
            .regex
            .capture_names()
            .flatten()
            .map(|name| {
                (
                    name.to_string(),
                    capt.name(name).unwrap().as_str().to_string(),
                )
            })
            .collect()
    }

    fn read_request(readable: &mut impl Read) -> anyhow::Result<Request> {
        let mut rdr = BufReader::new(readable);

        let mut first_line = String::new();
        rdr.read_line(&mut first_line)
            .context("Error while reading line")?;
        first_line = String::from(first_line.trim_ascii());

        let first_line_parts: Vec<&str> = first_line.split(" ").collect();

        let method;
        let url;

        match first_line_parts[..] {
            [method_raw, target, version] => {
                method = Method::from_str(method_raw).context("Unknown HTTP method")?;

                if !version.eq("HTTP/1.1") {
                    bail!("Unsupported HTTP version");
                }

                url = String::from(target);
            }
            _ => {
                bail!("Bad start-line");
            }
        }

        let mut headers: HashMap<String, String> = HashMap::new();
        let mut line = String::new();

        loop {
            line.clear();
            let n = rdr.read_line(&mut line).context("Can't read line")?;

            if n == 0 || line.trim_ascii().is_empty() {
                break;
            }

            let (k, v) = line
                .trim_ascii()
                .split_once(":")
                .ok_or(anyhow!("Invalid header"))?;
            headers.insert(k.trim_ascii().to_lowercase(), String::from(v.trim_ascii()));
        }

        let content = if let Some(content_length_raw) = headers.get("content-length") {
            let content_length: usize =
                content_length_raw.parse().context("Invalid header value")?;
            Self::read_content(&mut rdr, content_length)?
        } else {
            Vec::default()
        };

        let request = Request {
            method,
            url,
            headers,
            content,
        };
        Ok(request)
    }

    fn read_content(
        rdr: &mut BufReader<&mut impl Read>,
        mut content_length: usize,
    ) -> anyhow::Result<Vec<u8>> {
        let mut content = Vec::with_capacity(content_length);
        while content_length > 0 {
            let mut buf: [u8; BUFFER_SIZE] = [0u8; BUFFER_SIZE];
            let slice_to_read = &mut buf[..min(BUFFER_SIZE, content_length)];

            let bytes_read = rdr
                .read(slice_to_read)
                .context("Error while reading content")?;
            if bytes_read == 0 {
                break;
            } else {
                content.extend_from_slice(slice_to_read);
                content_length -= bytes_read;
            }
        }
        Ok(content)
    }
}

pub fn serialize_response(response: &Response) -> Vec<u8> {
    let content_len = response.content.as_ref().map(|c| c.len()).unwrap_or(0);
    let mut resp_bytes = Vec::with_capacity(content_len + response.headers.len() * 32);

    resp_bytes.extend(
        format!(
            "HTTP/1.1 {} {}\r\n",
            response.status.code_num, response.status.message
        )
        .as_bytes(),
    );

    for (key, value) in &response.headers {
        resp_bytes.extend(format!("{}: {}\r\n", key, value).as_bytes());
    }

    if let Some(c) = &response.content {
        resp_bytes.extend(format!("Content-Length: {}\r\n", c.len()).as_bytes());
        resp_bytes.extend("\r\n".as_bytes());
        resp_bytes.extend(c);
    } else {
        resp_bytes.extend("\r\n".as_bytes());
    }

    resp_bytes
}
