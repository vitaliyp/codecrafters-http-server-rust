use crate::http::encoding::Encoding::{Gzip, Identity};
use crate::http::encoding::{Encoding, EncodingVal};
use crate::http::middleware::Next;
use crate::http::request::RequestContext;
use crate::http::{Response, ok};
use anyhow::{Context, anyhow};
use once_cell::sync::Lazy;
use regex::Regex;
use std::collections::HashMap;

static ACCEPT_ENCODING_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"(?P<enc>[\w*\-]+)(?:;\s*q=(?P<q>0(\.\d+)?|1(\.0+)?))?").unwrap());

pub(in crate::http) fn parse_accept_encoding(
    header: &str,
) -> anyhow::Result<HashMap<Encoding, EncodingVal>> {
    ACCEPT_ENCODING_RE
        .captures_iter(header)
        .map(|cap| {
            let encoding = Encoding::try_from(
                cap.name("enc")
                    .ok_or(anyhow!("Cant get encoding from header value: {}", header))?
                    .as_str(),
            )?;

            let quality = cap
                .name("q")
                .map(|m| {
                    m.as_str()
                        .parse::<f32>()
                        .context("Failed to parse quality in header")
                })
                .unwrap_or(Ok(1.0))?;

            Ok((encoding, EncodingVal { encoding, quality }))
        })
        .collect()
}

pub trait Middleware: Send + Sync {
    fn handle(&self, ctx: &mut RequestContext, next: Next) -> Response;
}

pub struct CompressionMw {}

impl Middleware for CompressionMw {
    fn handle(&self, ctx: &mut RequestContext, next: Next) -> Response {
        let encodings = ctx
            .get_header("accept-encoding")
            .map(parse_accept_encoding)
            .unwrap_or_else(|| {
                Ok(HashMap::from([(
                    Identity,
                    EncodingVal {
                        encoding: Identity,
                        quality: 1.0,
                    },
                )]))
            });

        match encodings {
            Ok(encodings) => {
                let resp_encoding = if encodings.contains_key(&Gzip) {
                    Gzip
                } else {
                    Identity
                };

                let mut resp = next.run(ctx);
                resp.headers.insert(
                    "Content-Encoding".to_string(),
                    resp_encoding.to_string().to_lowercase(),
                );

                resp
            }
            Err(e) => {
                println!("{}", e);
                ok()
            }
        }
    }
}
