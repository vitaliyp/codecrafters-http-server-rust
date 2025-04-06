mod concurrency;
mod http;

use crate::http::{Request, Response, not_found, ok};
use http::{HttpCode, HttpMethod};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let dir = Arc::new(args.get(2).map(String::from));

    let mut server = http::Server::from_tcp_addr("127.0.0.1:4221").unwrap();

    server.add_handler(HttpMethod::GET, "/", Box::new(index));
    server.add_handler(HttpMethod::GET, "/echo/<s>", Box::new(echo));
    server.add_handler(HttpMethod::GET, "/user-agent", Box::new(user_agent));

    let dir_clone = Arc::clone(&dir);
    server.add_handler(
        HttpMethod::GET,
        "/files/<file>",
        Box::new(move |r| get_file(r, dir_clone.as_ref())),
    );

    let dir_clone = Arc::clone(&dir);
    server.add_handler(
        HttpMethod::POST,
        "/files/<file>",
        Box::new(move |r| post_file(r, dir_clone.as_ref())),
    );

    server.run().unwrap();
}

fn index(r: &Request) -> Response {
    ok()
}

fn echo(r: &Request) -> Response {
    let s = r.url_vars().get("s").unwrap();
    let headers = HashMap::from([("Content-Type", "text/plain")]);
    http::build_response(HttpCode::OK, &headers, &Some(s.as_bytes()))
}

fn user_agent(r: &Request) -> Response {
    let headers = HashMap::from([("Content-Type", "text/plain")]);
    http::build_response(
        HttpCode::OK,
        &headers,
        &Some(r.headers().get("user-agent").unwrap().as_bytes()),
    )
}

fn get_file(r: &Request, dir: &Option<String>) -> Response {
    if let Some(dir) = dir {
        let file_name = r.url_vars().get("file").unwrap();

        let mut file_path = PathBuf::from(dir);
        let file_name = Path::new(file_name);
        file_path.extend(file_name);

        let content = fs::read(file_path);
        match content {
            Ok(content) => http::build_response(
                HttpCode::OK,
                &HashMap::from([("Content-Type", "application/octet-stream")]),
                &Some(&content),
            ),
            Err(_) => not_found(),
        }
    } else {
        not_found()
    }
}

fn post_file(r: &Request, dir: &Option<String>) -> Response {
    if let Some(dir) = dir {
        let file_name = r.url_vars().get("file").unwrap();

        let mut path = PathBuf::from(dir);
        path.extend(Path::new(file_name));

        fs::write(path, &r.content()).unwrap();
        http::build_response(HttpCode::CREATED, &HashMap::new(), &None)
    } else {
        not_found()
    }
}
