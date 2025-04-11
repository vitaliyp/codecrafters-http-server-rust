use crate::http::encoding::{Encoding, EncodingVal};
use anyhow::{anyhow, Context};
use once_cell::sync::Lazy;
use regex::Regex;
use std::collections::HashMap;
use strum::ParseError;
use crate::http::encoding::Encoding::{Gzip, Identity};
use crate::http::middleware::{Middleware, Next};
use crate::http::request::RequestContext;
use crate::http::{ok, Response};

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
                if resp_encoding != Identity {
                    resp.headers.insert(
                        "Content-Encoding".to_string(),
                        resp_encoding.to_string().to_lowercase(),
                    );
                }
                resp
            }
            Err(e) => {
                println!("{}", e);
                ok()
            }
        }
    }
}

static ACCEPT_ENCODING_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"(?P<enc>[\w*\-]+)(?:;\s*q=(?P<q>0(\.\d+)?|1(\.0+)?))?").unwrap());

pub(in crate::http) fn parse_accept_encoding(
    header: &str,
) -> anyhow::Result<HashMap<Encoding, EncodingVal>> {
    ACCEPT_ENCODING_RE
        .captures_iter(header)
        .flat_map(|cap| {
            let encoding = Encoding::try_from(
                cap.name("enc").unwrap().as_str(),
            );

            let quality = cap
                .name("q")
                .map(|m| {
                    m.as_str()
                        .parse::<f32>().unwrap()
                })
                .unwrap_or(1.0);

            encoding.map(|encoding| Ok((encoding, EncodingVal { encoding, quality })))
        })
        .collect()
}


#[cfg(test)]
mod test {
    use crate::http::middleware::compression::ACCEPT_ENCODING_RE;

    #[test]
    fn test_re() {
        assert!(ACCEPT_ENCODING_RE.is_match("encoding-1"));

        let cc: Vec<&str> = ACCEPT_ENCODING_RE
            .captures_iter("encoding-1, gzip")
            .filter_map(|c| c.name("enc"))
            .map(|m| m.as_str()).collect();

        assert_eq!(cc, vec!["encoding-1", "gzip"])
    }
}
