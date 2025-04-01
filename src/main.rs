use std::io::{Read, Write};
#[allow(unused_imports)]
use std::net::TcpListener;
use regex::Regex;

fn main() {
    // You can use print statements as follows for debugging, they'll be visible when running tests.
    println!("Logs from your program will appear here!");

    let first_line_re = Regex::new(r"\S* (\S*) \S*").unwrap();

    let listener = TcpListener::bind("127.0.0.1:4221").unwrap();

    for stream in listener.incoming() {
        match stream {
            Ok(mut stream) => {
                println!("accepted new connection");

                let mut request = Vec::new();
                let mut buf: [u8; 1024] = [0; 1024];

                while let Ok(n) = stream.read(&mut buf) {
                    request.append(&mut buf[..n].to_vec());

                    if n < buf.len() {
                        break;
                    }
                }

                let request_string = String::from_utf8(request).unwrap();

                // println!("{:?}", &request_string);

                let mut msg_parts = request_string.split("\r\n");
                let first_line = msg_parts.next().unwrap();

                let captures = first_line_re.captures(first_line).unwrap();

                let url = captures.get(1).unwrap().as_str();

                match url {
                    "/" => {
                        stream.write("HTTP/1.1 200 OK\r\n\r\n".as_bytes()).unwrap();
                    }
                    _ => {
                        stream.write("HTTP/1.1 404 Not Found\r\n\r\n".as_bytes()).unwrap();
                    }
                }
            }
            Err(e) => {
                println!("error: {}", e);
            }
        }
    }
}
