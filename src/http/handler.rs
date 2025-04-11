use crate::http::Response;
use crate::http::request::RequestContext;

pub type HandlerFunc = Box<dyn Fn(&RequestContext) -> Response + Sync + Send>;
