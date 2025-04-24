use url::{ParseError, Url};

pub fn to_url(host: String) -> Result<Url, ParseError> {
    Url::parse(&host)
}
