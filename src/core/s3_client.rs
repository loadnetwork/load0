use crate::utils::get_env::get_env_var;
use anyhow::Error;
use aws_config::Region;
use aws_sdk_s3::{Client, Config, config::Credentials, primitives::ByteStream};

pub async fn init_s3_client() -> Result<Client, Error> {
    let endpoint_uri = get_env_var("SUPABASE_URL_SDK")?;
    let access_key_id = get_env_var("AWS_ACCESS_KEY_ID")?;
    let secret_access_key = get_env_var("AWS_SECRET_ACCESS_KEY")?;
    let region = get_env_var("AWS_REGION")?;

    let credentials = Credentials::new(
        access_key_id,
        secret_access_key,
        None, // session token
        None, // expiration
        "load1-node",
    );

    let config = Config::builder()
        .region(Region::new(region))
        .endpoint_url(endpoint_uri)
        .force_path_style(true)
        .credentials_provider(credentials)
        .behavior_version_latest()
        .build();

    let client = Client::from_conf(config);

    Ok(client)
}
