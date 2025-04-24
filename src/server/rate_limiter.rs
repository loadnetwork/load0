use crate::utils::get_env::env_var_to_vec;
use crate::utils::urls::to_url;
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

pub fn whitelisted_urls() -> Vec<String> {
    env_var_to_vec("WHITELISTED_HOSTS")
}

pub fn is_whitelisted(host: Option<String>, whitelisted_domains: &Vec<String>) -> bool {
    match host {
        None => false,
        Some(host) => {
            let url = to_url(host);
            if let Ok(url) = url {
                let host = url.host_str().unwrap_or("");
                let host_string = String::from(host);
                whitelisted_domains.contains(&host_string)
            } else {
                false
            }
        }
    }
}

#[cfg(test)]
mod cfg_tests {
    use crate::server::rate_limiter::{is_whitelisted, whitelisted_urls};

    #[test]
    pub fn test_whitelisted_function() {
        let whitelisted_domains = vec![
            String::from("cloud.load.network"),
            String::from("localhost"),
            String::from("relic.bot"),
        ];

        assert!(is_whitelisted(
            Some("http://cloud.load.network".to_string()),
            &whitelisted_domains
        ));
        assert!(is_whitelisted(
            Some("http://localhost".to_string()),
            &whitelisted_domains
        ));
        assert!(is_whitelisted(
            Some("http://relic.bot".to_string()),
            &whitelisted_domains
        ));
        assert!(is_whitelisted(
            Some("https://relic.bot".to_string()),
            &whitelisted_domains
        ));
        assert!(is_whitelisted(
            Some("https://localhost".to_string()),
            &whitelisted_domains
        ));
        assert!(!is_whitelisted(
            Some("https://facebook.com".to_string()),
            &whitelisted_domains
        ));
        assert!(!is_whitelisted(None, &whitelisted_domains));

        unsafe {
            std::env::set_var("WHITELISTED_HOSTS", "facebook.com,google.com");
        }

        let whitelisted_domains = whitelisted_urls();
        assert!(is_whitelisted(
            Some("https://facebook.com".to_string()),
            &whitelisted_domains
        ));
        assert!(is_whitelisted(
            Some("https://google.com".to_string()),
            &whitelisted_domains
        ));
        assert!(!is_whitelisted(
            Some("https://localhost".to_string()),
            &whitelisted_domains
        ));
    }
}
