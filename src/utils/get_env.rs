use anyhow::Error;
use dotenv::dotenv;
use std::env;

pub fn get_env_var(key: &str) -> Result<String, Error> {
    dotenv().ok();
    Ok(env::var(key)?)
}
