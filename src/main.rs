use std::collections::HashMap;
use std::fmt::Write as _;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpListener;
use std::str::FromStr;
use std::time::Duration;

impl HttpCode {
    const OK: HttpCode = HttpCode {
        code_num: 200,
        message: "OK",
    };
    const BAD_REQUEST: HttpCode = HttpCode {
        code_num: 400,
        message: "Bad Request",
    };
    const NOT_FOUND: HttpCode = HttpCode {
        code_num: 404,
        message: "Not Found",
    };
}

struct HttpCode {
    code_num: u16,
    message: &'static str,
}

#[derive(Debug)]
enum HttpMethod {
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
struct Request {
    method: HttpMethod,
    url: String,
    headers: HashMap<String, String>,
    content: Vec<u8>,
}

fn read_request(readable: &mut impl Read) -> Result<Request, String> {
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
        headers.insert(String::from(k.trim_ascii().to_lowercase()), String::from(v.trim_ascii()));
    }

    Ok(Request {
        method,
        url,
        headers,
        content: Vec::default(),
    })
}

fn main() {
    let listener = TcpListener::bind("127.0.0.1:4221").unwrap();

    for stream in listener.incoming() {
        match stream {
            Ok(mut stream) => {
                stream
                    .set_read_timeout(Some(Duration::from_secs(5)))
                    .unwrap();

                println!("accepted new connection: {:?}", stream.peer_addr());

                let request = read_request(&mut stream).unwrap();
                println!("{request:?}");

                let url_parts = parse_url(&request.url);

                stream
                    .write(
                        match (request.method, &url_parts[..]) {
                            (HttpMethod::GET, [""]) => {
                                build_response(HttpCode::OK, &HashMap::new(), &None)
                            }
                            (HttpMethod::GET, ["echo", s]) => {
                                let headers = HashMap::from([("Content-Type", "text/plain")]);
                                build_response(HttpCode::OK, &headers, &Some(s))
                            }
                            (HttpMethod::GET, ["user-agent"]) => {
                                let headers = HashMap::from([("Content-Type", "text/plain")]);
                                build_response(HttpCode::OK, &headers, 
                                               &Some(request.headers.get("user-agent").unwrap())
                                )
                            }
                            _ => build_response(HttpCode::NOT_FOUND, &HashMap::new(), &None),
                        }
                        .as_bytes(),
                    )
                    .unwrap();
            }
            Err(e) => {
                println!("error: {}", e);
            }
        }
    }
}

fn parse_url(url: &str) -> Vec<&str> {
    url.strip_prefix("/").unwrap().split("/").collect()
}

fn build_response(code: HttpCode, headers: &HashMap<&str, &str>, content: &Option<&str>) -> String {
    let content_len: usize = content.map(str::len).iter().sum();
    let mut response = String::with_capacity(content_len + headers.len() * 32);

    write!(response, "HTTP/1.1 {} {}\r\n", code.code_num, code.message).unwrap();

    for (key, value) in headers {
        write!(response, "{}: {}\r\n", key, value).unwrap();
    }

    if let Some(c) = content {
        write!(response, "Content-Length: {}\r\n", c.len()).unwrap();
        write!(response, "\r\n{}", c).unwrap();
    }

    response.push_str("\r\n");

    response
}
