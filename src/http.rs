mod encoding;
mod handler;
pub mod method;
mod middleware;
pub mod request;
pub mod server;
pub mod status;

use status::Status;
use std::collections::HashMap;

const BUFFER_SIZE: usize = 1024;

pub struct Response {
    status: Status,
    headers: HashMap<String, String>,
    content: Option<Vec<u8>>,
}

impl Response {
    pub(crate) fn from_parts(
        status: Status,
        headers: HashMap<String, String>,
        content: Option<Vec<u8>>,
    ) -> Self {
        Self {
            status,
            headers,
            content,
        }
    }
}

pub fn ok() -> Response {
    Response::from_parts(Status::OK, HashMap::new(), None)
}

pub fn not_found() -> Response {
    Response::from_parts(Status::NOT_FOUND, HashMap::new(), None)
}

pub fn bad_request() -> Response {
    Response::from_parts(Status::BAD_REQUEST, HashMap::new(), None)
}
