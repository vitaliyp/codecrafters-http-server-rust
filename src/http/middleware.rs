use crate::http::handler::HandlerFunc;
use crate::http::Response;
use crate::http::request::RequestContext;

pub struct Next<'a> {
    pub(crate) middlewares: &'a [Box<dyn Middleware>],
    pub(crate) handler: &'a HandlerFunc,
}

impl<'a> Next<'a> {
    pub fn run(self, ctx: &mut RequestContext) -> Response {
        if let Some((first, rest)) = self.middlewares.split_first() {
            let next = Next {
                middlewares: rest,
                handler: self.handler,
            };
            first.handle(ctx, next)
        } else {
            (self.handler)(ctx)
        }
    }
}

pub trait Middleware: Send + Sync {
    fn handle(&self, ctx: &mut RequestContext, next: Next) -> Response;
}
