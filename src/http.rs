use std::collections::HashMap;
use std::fmt::Write as _;
use std::io::{BufRead, BufReader, Read};
use std::str::FromStr;

pub struct HttpCode {
    pub code_num: u16,
    pub message: &'static str,
}

impl HttpCode {
    pub const OK: HttpCode = HttpCode {
        code_num: 200,
        message: "OK",
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

#[derive(Debug)]
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
pub struct Request {
    pub method: HttpMethod,
    pub url: String,
    pub headers: HashMap<String, String>,
    pub content: Vec<u8>,
}

pub fn read_request(readable: &mut impl Read) -> Result<Request, String> {
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

    Ok(Request {
        method,
        url,
        headers,
        content: Vec::default(),
    })
}

pub fn parse_url(url: &str) -> Vec<&str> {
    url.strip_prefix("/").unwrap().split("/").collect()
}

pub fn build_response(
    code: HttpCode,
    headers: &HashMap<&str, &str>,
    content: &Option<&str>,
) -> String {
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
