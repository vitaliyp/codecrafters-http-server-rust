use std::collections::HashMap;
use crate::http::encoding::{Encoding, EncodingVal};
use anyhow::{Context, anyhow};
use once_cell::sync::Lazy;
use regex::Regex;
static ACCEPT_ENCODING_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"(?P<enc>[\w*\-]+)(?:;\s*q=(?P<q>0(\.\d+)?|1(\.0+)?))?").unwrap());

pub(super) fn parse_accept_encoding(header: &str) -> anyhow::Result<HashMap<Encoding, EncodingVal>> {
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

// pub(super) fn decide_encoding(internal_req: RequestContext) -> Encoding {
//     if internal_req.encodings.contains_key(&Gzip) {
//         Gzip
//     } else { 
//         Identity
//     }
// }
