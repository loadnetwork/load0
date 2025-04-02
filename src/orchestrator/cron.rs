use bundler::utils::core::large_bundle::LargeBundle;
use reqwest::Client;

use crate::core::bundler_superaccount::init_superaccount;
use crate::orchestrator::db::{get_unsettled_bundles, update_bundle_settled_status};
use crate::utils::constants::FOUR_MB;
use crate::utils::get_env::get_env_var;
use anyhow::{Error, anyhow};

pub async fn update() -> Result<(), Error> {
    let funder_pk = get_env_var("SUPERACCOUNT_PK")?;
    let unsettled_bundles_count = get_unsettled_bundles().await.unwrap_or(Vec::new());
    if (unsettled_bundles_count.len() == 0) {
        return Ok(());
    }

    let unsettled_bundles = get_unsettled_bundles().await?;
    println!("UNSETTLED BUNDLES: {:?}", unsettled_bundles);
    let header_bundle = unsettled_bundles
    .get(0)
    .ok_or_else(|| anyhow!("Error getting unsettled bundles"))?;

    if (unsettled_bundles.len() == 0 || header_bundle.data_size == 0) {
        return Ok(());
    }

    let header_bundle_obj = get_optimistic_bundle_data(&header_bundle.optimistic_hash).await?;
    let header_bundle_data = header_bundle_obj.0.clone();
    let header_bundle_size = header_bundle_obj.0.len() as f64;
    let header_bundle_mime = header_bundle_obj.1;
    let super_account = init_superaccount().await?;
    let chunkers_count = (header_bundle_size as f64 / FOUR_MB as f64).ceil() as u32;

    let large_bundle = LargeBundle::new()
        .data(header_bundle_data)
        .private_key(funder_pk)
        .content_type(header_bundle_mime)
        .super_account(super_account)
        .with_chunkers_count(chunkers_count)
        .chunk()
        .build()?
        .propagate_chunks()
        .await?
        .finalize()
        .await?;
    println!("large bundle broadcasted: {:?}", large_bundle);

    let _ = update_bundle_settled_status(&header_bundle.optimistic_hash, true, &large_bundle)
        .await
        .unwrap();
    Ok(())
}

async fn get_optimistic_bundle_data(optimistic_hash: &str) -> Result<(Vec<u8>, String), Error> {
    let supabase_url = get_env_var("SUPABASE_URL")?;
    println!("{:?}", supabase_url);
    let api_key = get_env_var("SUPABASE_API_KEY")?;
    println!("{:?}", api_key);
    let bucket_name = get_env_var("AWS_BUCKET_NAME")?;
    println!("{:?}", bucket_name);
    let http_client = Client::new();

    let direct_url = format!(
        "{}/object/public/{}/{}",
        supabase_url.replace("/v1/s3", "/v1"),
        bucket_name,
        optimistic_hash
    );

    let file_response = http_client
        .get(&direct_url)
        .header("apikey", &api_key)
        .header("Authorization", format!("Bearer {}", api_key))
        .send()
        .await?;

    // if !file_response.status().is_success() {
    //     let status = file_response.status();
    //     let error_text = file_response.text().await.unwrap_or_default();
    //     println!("Error accessing file: {} - {}", status, error_text);
    //     // TODO: return error
    // }
    let content_type = file_response
        .headers()
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("application/octet-stream")
        .to_string();

    let bytes = file_response.bytes().await?.to_vec();
    Ok((bytes, content_type))
}
