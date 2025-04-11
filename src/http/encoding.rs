use strum::{Display, EnumString, IntoStaticStr};

#[derive(EnumString, IntoStaticStr, Debug, PartialEq, Eq, Hash, Clone, Copy, Display)]
pub enum Encoding {
    #[strum(serialize = "gzip")]
    Gzip,
    #[strum(serialize = "compress")]
    Compress,
    #[strum(serialize = "deflate")]
    Deflate,
    #[strum(serialize = "br")]
    Br,
    #[strum(serialize = "zstd")]
    Zstd,
    #[strum(serialize = "dcb")]
    Dcb,
    #[strum(serialize = "dcz")]
    Dcz,

    #[strum(serialize = "identity")]
    Identity,
    #[strum(serialize = "*")]
    Any,
}

#[derive(Debug, PartialEq)]
pub struct EncodingVal {
    pub encoding: Encoding,
    pub quality: f32,
}
