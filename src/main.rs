mod concurrency;
mod http;

use crate::http::{not_found, ok, Response};
use http::method::Method;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use http::server;
use http::status::Status;
use crate::http::request::RequestContext;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let dir = Arc::new(args.get(2).map(String::from));

    let mut server = server::Server::from_tcp_addr("127.0.0.1:4221", 10).unwrap();

    server.add_handler(Method::GET, "/", Box::new(index));
    server.add_handler(Method::GET, "/echo/<s>", Box::new(echo));
    server.add_handler(Method::GET, "/user-agent", Box::new(user_agent));

    let dir_clone = Arc::clone(&dir);
    server.add_handler(
        Method::GET,
        "/files/<file>",
        Box::new(move |r| get_file(r, dir_clone.as_ref())),
    );

    let dir_clone = Arc::clone(&dir);
    server.add_handler(
        Method::POST,
        "/files/<file>",
        Box::new(move |r| post_file(r, dir_clone.as_ref())),
    );

    server.run().unwrap();
}

fn index(_r: &RequestContext) -> Response {
    ok()
}

fn echo(r: &RequestContext) -> Response {
    let s = r.get_var("s").unwrap();
    
    let headers = HashMap::from(
        [("Content-Type".to_string(), "text/plain".to_string())]);
    
    Response::from_parts(Status::OK, headers, Some(s.as_bytes().to_vec()))
}

fn user_agent(r: &RequestContext) -> Response {
    let headers = HashMap::from(
        [("Content-Type".to_string(), "text/plain".to_string())]);
    
    Response::from_parts(Status::OK, headers,
        Some(r.get_header("user-agent").unwrap().as_bytes().to_vec()))
}

fn get_file(r: &RequestContext, dir: &Option<String>) -> Response {
    let dir = match dir {
        Some(d) => d,
        None => return not_found(),
    };

    let file_name = r.get_var("file").unwrap();

    let mut file_path = PathBuf::from(dir);
    file_path.push(file_name);

    match fs::read(file_path) {
        Ok(content) => Response::from_parts(
            Status::OK,
            HashMap::from([
                ("Content-Type".to_string(), "application/octet-stream".to_string())
            ]),
            Some(content),
        ),
        Err(_) => not_found(),
    }
}

fn post_file(r: &RequestContext, dir: &Option<String>) -> Response {
    let dir = match dir {
        Some(d) => d,
        None => return not_found(),
    };
    
    let file_name = r.get_var("file").unwrap();

    let mut path = PathBuf::from(dir);
    path.extend(Path::new(file_name));

    fs::write(path, &r.request().content).unwrap();
    Response::from_parts(Status::CREATED, HashMap::new(), None)
}
