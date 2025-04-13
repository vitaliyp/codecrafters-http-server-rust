use crate::http::method::Method;
use std::any::{Any, TypeId};
use std::collections::HashMap;

#[derive(Debug)]
pub(crate) struct RequestContext<'a> {
    request: &'a Request,
    extensions: HashMap<TypeId, Box<dyn Any + Send + Sync>>,
    url_vars: HashMap<String, String>,
}

impl RequestContext<'_> {
    pub fn from(request: &Request, url_vars: HashMap<String, String>) -> RequestContext {
        RequestContext {
            request,
            extensions: HashMap::new(),
            url_vars,
        }
    }

    pub fn get_var(&self, k: &str) -> Option<&str> {
        self.url_vars.get(k).map(|v| v.as_str())
    }

    pub fn get_header(&self, k: &str) -> Option<&str> {
        self.request
            .headers
            .get(&k.to_lowercase())
            .map(|v| v.as_str())
    }

    pub fn request(&self) -> &Request {
        self.request
    }
}

#[derive(Debug)]
pub struct Request {
    pub method: Method,
    pub url: String,
    pub headers: HashMap<String, String>,
    pub content: Vec<u8>,
}

impl Request {
    pub fn get_header(&self, k: &str) -> Option<&str> {
        self.headers.get(&k.to_lowercase()).map(|v| v.as_str())
    }
}
