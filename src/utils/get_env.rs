use anyhow::Error;
use dotenv::dotenv;
use std::env;

pub fn get_env_var(key: &str) -> Result<String, Error> {
    dotenv().ok();
    Ok(env::var(key)?)
}

pub fn env_var_to_vec(key: &str) -> Vec<String> {
    match std::env::var(key) {
        Ok(raw) if !raw.trim().is_empty() => raw
            .split(',')
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
            .map(String::from)
            .collect(),
        _ => Vec::new(),
    }
}
