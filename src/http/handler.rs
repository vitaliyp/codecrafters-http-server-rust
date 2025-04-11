use crate::http::request::{Request, RequestContext};
use crate::http::Response;

// pub trait Handler: Send + Sync {
//     fn handle(&self, ctx: &RequestContext) -> Response;
// }

pub type HandlerFunc = Box<dyn Fn(&RequestContext) -> Response + Sync + Send + 'static>;
