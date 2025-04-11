use strum::EnumString;

#[derive(EnumString, Debug, PartialEq)]
pub enum Method {
    #[strum(serialize = "GET")]
    GET,
    #[strum(serialize = "POST")]
    POST,
}
