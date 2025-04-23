use axum::extract::ConnectInfo;
use axum::http::Request;
use std::net::SocketAddr;
use tower_governor::GovernorError;
use tower_governor::key_extractor::KeyExtractor;

pub const LOAD_HEADER_NAME: &str = "X-Load-Authorization";

#[derive(Clone)]
pub struct XLoadAuthHeaderExtractor;

impl KeyExtractor for XLoadAuthHeaderExtractor {
    type Key = String;

    fn extract<T>(&self, req: &Request<T>) -> Result<Self::Key, GovernorError> {
        let ip = req.extensions().get::<ConnectInfo<SocketAddr>>();
        if let Some(ip) = ip {
            let headers = req.headers();

            match headers.get(LOAD_HEADER_NAME) {
                Some(res) => {
                    let res = res
                        .to_str()
                        .map_err(|_| GovernorError::UnableToExtractKey)?;

                    Ok(res.to_owned())
                }
                None => Ok(ip.0.ip().to_string()),
            }
        } else {
            Err(GovernorError::UnableToExtractKey)
        }
    }
}
