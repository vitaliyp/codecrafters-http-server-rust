use crate::concurrency::ThreadPool;
use std::cmp::{min, PartialEq};
use std::collections::HashMap;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::str::FromStr;
use std::sync::Arc;
use std::time::Duration;
use once_cell::sync::Lazy;
use regex::{Captures, Regex};

const BUFFER_SIZE: usize = 1024;

pub struct HttpCode {
    pub code_num: u16,
    pub message: &'static str,
}

impl HttpCode {
    pub const OK: HttpCode = HttpCode {
        code_num: 200,
        message: "OK",
    };
    pub const CREATED: HttpCode = HttpCode {
        code_num: 201,
        message: "Created",
    };
    pub const BAD_REQUEST: HttpCode = HttpCode {
        code_num: 400,
        message: "Bad Request",
    };
    pub const NOT_FOUND: HttpCode = HttpCode {
        code_num: 404,
        message: "Not Found",
    };
}

#[derive(Debug, PartialEq)]
pub enum HttpMethod {
    GET,
    POST,
}

impl FromStr for HttpMethod {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "GET" => Ok(HttpMethod::GET),
            "POST" => Ok(HttpMethod::POST),
            _ => Err(()),
        }
    }
}

#[derive(Debug)]
struct InternalReq {
    method: HttpMethod,
    url: String,
    headers: HashMap<String, String>,
    content: Vec<u8>,
}


pub struct Request {
    internal: InternalReq,
    url_vars: HashMap<String, String>
}

impl Request {
    pub fn headers(&self) -> &HashMap<String, String> {
        &self.internal.headers
    }

    pub fn method(&self) -> &HttpMethod {
        &self.internal.method
    }

    pub fn content(&self) -> &[u8] {
        &self.internal.content
    }

    pub fn url_vars(&self) -> &HashMap<String, String> {
        &self.url_vars
    }

}

pub struct Response(pub Vec<u8>);

type HandlerFunc = Box<dyn Fn(&Request) -> Response + Sync + Send + 'static>;

pub struct Handler {
    method: HttpMethod,
    regex: Regex,
    f: HandlerFunc,
}

pub struct Server {
    listener: TcpListener,
    handlers: Vec<Handler>,
    pool: ThreadPool,
}


impl Server {
    fn new(listener: TcpListener) -> Server {
        Server {
            listener,
            handlers: Vec::new(),
            pool: ThreadPool::new(10),
        }
    }

    pub fn from_tcp_addr(addr: &str) -> Result<Server, &'static str> {
        let listener = TcpListener::bind(addr).map_err(|_| "Can't bind address")?;
        Ok(Server::new(
            listener,
        ))
    }

    const PATTERN_RE: Lazy<Regex> = Lazy::new(|| Regex::new(r"<(?P<var>[a-z][a-z0-9]*)>").unwrap());

    pub fn add_handler(&mut self, m: HttpMethod, pattern: &str, f: HandlerFunc) {
        let pattern = Self::PATTERN_RE.replace_all(pattern, |capt: &Captures|{
            format!(r"(?<{}>[^/?]+)", capt.name("var").unwrap().as_str())
        }).to_string();
        let pattern = format!("^{}$", pattern);

        let compiled = Regex::new(&pattern).unwrap();

        self.handlers.push(Handler {
            method: m,
            regex: compiled,
            f
        })
    }

    pub fn run(self) -> Result<(), &'static str> {
        let server = Arc::new(self);
        for stream in server.listener.incoming() {
            let stream = stream.map_err(|_| "Error listening")?;
            let thread_server = Arc::clone(&server);
            server.pool.execute(move || thread_server.process_incoming(stream));
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
            bad_request()
        };

        stream.write_all(&response.0).unwrap();
    }

    fn dispatch(&self, req: InternalReq) -> Response {
        let handler = self.handlers.iter()
            .map(|h| (h, h.regex.captures(&req.url)))
            .filter(|(h, c)| h.method == req.method && c.is_some())
            .next();

        if let Some((handler, Some(capt))) = handler {
            let url_vars: HashMap<String, String> = handler.regex.capture_names()
                .flatten()
                .map(|name| (name.to_string(), capt.name(name).unwrap().as_str().to_string()))
                .collect();

            let request = Request {
                internal: req,
                url_vars,
            };

            (handler.f)(&request)
        } else {
            not_found()
        }
    }


    fn read_request(readable: &mut impl Read) -> Result<InternalReq, String> {
        let mut rdr = BufReader::new(readable);

        let mut first_line = String::new();
        rdr.read_line(&mut first_line)
            .map_err(|_| "Error while reading line")?;
        first_line = String::from(first_line.trim_ascii());

        let first_line_parts: Vec<&str> = first_line.split(" ").collect();

        let method;
        let url;

        match first_line_parts[..] {
            [method_raw, target, version] => {
                method = HttpMethod::from_str(method_raw).map_err(|_| "Unknown HTTP method")?;

                if !version.eq("HTTP/1.1") {
                    return Err(String::from("Unsupported HTTP version"));
                }

                url = String::from(target);
            }
            _ => {
                return Err(String::from("Bad start-line"));
            }
        }

        let mut headers: HashMap<String, String> = HashMap::new();
        let mut line = String::new();

        loop {
            line.clear();
            let n = rdr
                .read_line(&mut line)
                .map_err(|e| format!("Can't read line: {}", e))?;

            if n == 0 || line.trim_ascii().is_empty() {
                break;
            }

            let (k, v) = line.trim_ascii().split_once(":").ok_or("Invalid header")?;
            headers.insert(
                String::from(k.trim_ascii().to_lowercase()),
                String::from(v.trim_ascii()),
            );
        }

        let content = if let Some(content_length_raw) = headers.get("content-length") {
            let content_length: usize = content_length_raw.parse().map_err(|_| "Invalid header value")?;
            Self::read_content(&mut rdr, content_length)?
        } else {
            Vec::default()
        };

        let request = InternalReq {
            method,
            url,
            headers,
            content,
        };
        Ok(request)
    }

    fn read_content(rdr: &mut BufReader<&mut impl Read>, mut content_length: usize) -> Result<Vec<u8>, String> {
        let mut content = Vec::with_capacity(content_length);
        while content_length > 0 {
            let mut buf: [u8; BUFFER_SIZE] = [0u8; BUFFER_SIZE];
            let slice_to_read = &mut buf[..min(BUFFER_SIZE, content_length)];

            let bytes_read = rdr
                .read(slice_to_read)
                .map_err(|_| "Error while reading content")?;
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

pub fn build_response(
    code: HttpCode,
    headers: &HashMap<&str, &str>,
    content: &Option<&[u8]>,
) -> Response {
    let content_len: usize = content.map(|v| v.len()).iter().sum();
    let mut response = Vec::with_capacity(content_len + headers.len() * 32);

    response.extend(format!("HTTP/1.1 {} {}\r\n", code.code_num, code.message).as_bytes());

    for (key, value) in headers {
        response.extend(format!("{}: {}\r\n", key, value).as_bytes());
    }

    if let Some(c) = content {
        response.extend(format!("Content-Length: {}\r\n", c.len()).as_bytes());
        response.extend("\r\n".as_bytes());
        response.extend(*c);
    } else {
        response.extend("\r\n".as_bytes());
    }

    Response(response)
}

pub fn ok() -> Response {
    build_response(HttpCode::OK, &HashMap::new(), &None)
}

pub fn not_found() -> Response {
    build_response(HttpCode::NOT_FOUND, &HashMap::new(), &None)
}

pub fn bad_request() -> Response {
    build_response(HttpCode::BAD_REQUEST, &HashMap::new(), &None)
}
