mod concurrency;
mod http;

use concurrency::ThreadPool;
use http::Request;
use http::{HttpCode, HttpMethod};
use std::collections::HashMap;
use std::fs;
use std::io::Write;
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::time::Duration;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let dir = &args.get(2).map(String::from);

    let pool = ThreadPool::new(10);
    let listener = TcpListener::bind("127.0.0.1:4221").unwrap();

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                let thread_dir = dir.clone();
                pool.execute(|| process_incoming(stream, thread_dir));
            }
            Err(_) => {
                println!("Error");
            }
        }
    }
}

fn process_incoming(mut stream: TcpStream, dir: Option<String>) {
    stream
        .set_read_timeout(Some(Duration::from_secs(5)))
        .unwrap();

    println!("accepted new connection: {:?}", stream.peer_addr());

    let request = http::read_request(&mut stream).unwrap();
    println!("{request:?}");

    let response = dispatch(&request, &dir);

    stream.write(&response).unwrap();
}

fn dispatch(request: &Request, dir: &Option<String>) -> Vec<u8> {
    let url_parts = http::parse_url(&request.url);

    match (&request.method, &url_parts[..]) {
        (HttpMethod::GET, [""]) => http::build_response(HttpCode::OK, &HashMap::new(), &None),

        (HttpMethod::GET, ["echo", s]) => {
            let headers = HashMap::from([("Content-Type", "text/plain")]);
            http::build_response(HttpCode::OK, &headers, &Some(s.as_bytes()))
        }

        (HttpMethod::GET, ["user-agent"]) => {
            let headers = HashMap::from([("Content-Type", "text/plain")]);
            http::build_response(
                HttpCode::OK,
                &headers,
                &Some(request.headers.get("user-agent").unwrap().as_bytes()),
            )
        }
        (HttpMethod::GET, ["files", file_name]) => {
            if dir.is_none() {
                return http::build_response(HttpCode::NOT_FOUND, &HashMap::new(), &None)
            }
            
            let dir = dir.as_ref().unwrap();
            
            let mut file_path = PathBuf::from(dir);
            let file_name = Path::new(file_name);
            file_path.extend(file_name);
            
            let content = fs::read(file_path);
            match content {
                Ok(content) => {
                    http::build_response(
                        HttpCode::OK,
                        &HashMap::from([("Content-Type", "application/octet-stream")]),
                        &Some(&content),
                    )
                }
                Err(_) => {
                    http::build_response(HttpCode::NOT_FOUND, &HashMap::new(), &None)
                }
            }
        }
        _ => http::build_response(HttpCode::NOT_FOUND, &HashMap::new(), &None),
    }
}
