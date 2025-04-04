mod concurrency;
mod http;

use http::Request;
use http::{HttpCode, HttpMethod};
use std::collections::HashMap;
use std::io::Write;
use std::net::{TcpListener, TcpStream};
use std::time::Duration;
use concurrency::ThreadPool;

fn main() {
    let pool = ThreadPool::new(10);
    let listener = TcpListener::bind("127.0.0.1:4221").unwrap();

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                pool.execute(|| process_incoming(stream));
            }
            Err(_) => {
                println!("Error");
            }
        }
    }
}

fn process_incoming(mut stream: TcpStream) {
    stream
        .set_read_timeout(Some(Duration::from_secs(5)))
        .unwrap();

    println!("accepted new connection: {:?}", stream.peer_addr());

    let request = http::read_request(&mut stream).unwrap();
    println!("{request:?}");

    let response = dispatch(&request);

    stream.write(response.as_bytes()).unwrap();
}

fn dispatch(request: &Request) -> String {
    let url_parts = http::parse_url(&request.url);

    match (&request.method, &url_parts[..]) {
        (HttpMethod::GET, [""]) => http::build_response(HttpCode::OK, &HashMap::new(), &None),

        (HttpMethod::GET, ["echo", s]) => {
            let headers = HashMap::from([("Content-Type", "text/plain")]);
            http::build_response(HttpCode::OK, &headers, &Some(s))
        }

        (HttpMethod::GET, ["user-agent"]) => {
            let headers = HashMap::from([("Content-Type", "text/plain")]);
            http::build_response(
                HttpCode::OK,
                &headers,
                &Some(request.headers.get("user-agent").unwrap()),
            )
        }
        _ => http::build_response(HttpCode::NOT_FOUND, &HashMap::new(), &None),
    }
}
