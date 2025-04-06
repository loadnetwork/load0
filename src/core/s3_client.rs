use crate::utils::get_env::get_env_var;
use anyhow::Error;
use aws_config::Region;
use aws_config::retry::RetryConfig;
use aws_config::timeout::TimeoutConfig;
use aws_sdk_s3::{Client, Config, config::Credentials};
use std::time::Duration;

pub async fn init_s3_client() -> Result<Client, Error> {
    let endpoint_uri = get_env_var("SUPABASE_URL_SDK")?;
    let access_key_id = get_env_var("S3_ACCESS_KEY_ID")?;
    let secret_access_key = get_env_var("S3_SECRET_ACCESS_KEY")?;
    let region = get_env_var("S3_REGION")?;

    let credentials = Credentials::new(
        access_key_id,
        secret_access_key,
        None, // session token
        None, // expiration
        "load0-node",
    );

    let retry_config = RetryConfig::standard()
        .with_max_attempts(5)
        .with_initial_backoff(Duration::from_millis(100))
        .with_max_backoff(Duration::from_secs(5));

    let timeout_config = TimeoutConfig::builder()
        .operation_timeout(Duration::from_secs(3600))
        .operation_attempt_timeout(Duration::from_secs(300))
        .build();

    let config = Config::builder()
        .region(Region::new(region))
        .endpoint_url(endpoint_uri)
        .force_path_style(true)
        .credentials_provider(credentials)
        .retry_config(retry_config)
        .timeout_config(timeout_config)
        .behavior_version_latest()
        .build();

    let client = Client::from_conf(config);

    Ok(client)
}
