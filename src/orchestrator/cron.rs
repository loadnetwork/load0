use crate::core::bundler_superaccount::init_superaccount;
use crate::orchestrator::db::{get_unsettled_bundles, update_bundle_settled_status};
use crate::utils::constants::FOUR_MB;
use crate::utils::get_env::get_env_var;
use anyhow::{Error, anyhow};
use bundler::utils::core::large_bundle::LargeBundle;

pub async fn update() -> Result<(), Error> {
    let funder_pk = get_env_var("SUPERACCOUNT_PK").unwrap();
    let unsettled_bundles = get_unsettled_bundles().await.unwrap_or_default();
    if unsettled_bundles.is_empty() {
        println!("No unsettled bundles to process");
        return Ok(());
    }

    println!("UNSETTLED BUNDLES: {:?}", unsettled_bundles);
    let header_bundle = unsettled_bundles
        .get(0)
        .ok_or_else(|| anyhow!("Error getting unsettled bundles"))?;

    if header_bundle.data_size == 0 {
        println!("Bundle has zero data size, skipping");
        return Ok(());
    }
    let header_bundle_obj = get_optimistic_bundle_data(&header_bundle.optimistic_hash).await?;

    let header_bundle_data = header_bundle_obj.0.clone();
    let header_bundle_size = header_bundle_obj.0.len() as f64;
    let header_bundle_mime = header_bundle_obj.1;

    let super_account = init_superaccount().await?;

    let chunkers_count = (header_bundle_size as f64 / FOUR_MB as f64).ceil() as u32;
    println!("Processing bundle with {} chunks", chunkers_count);

    let large_bundle_builder = LargeBundle::new()
        .data(header_bundle_data)
        .private_key(funder_pk)
        .content_type(header_bundle_mime)
        .super_account(super_account)
        .with_chunkers_count(chunkers_count)
        .chunk()
        .build()
        .map_err(|e| anyhow!("Error building large bundle: {:?}", e))?;

    // propagate chunks
    let propagated = large_bundle_builder
        .propagate_chunks()
        .await
        .map_err(|e| anyhow!("Error propagating chunks: {:?}", e))?;

    let large_bundle = propagated
        .finalize()
        .await
        .map_err(|e| anyhow!("Error finalizing bundle: {:?}", e))?;

    update_bundle_settled_status(&header_bundle.optimistic_hash, true, &large_bundle).await?;

    println!("Successfully updated bundle status");
    Ok(())
}

async fn get_optimistic_bundle_data(optimistic_hash: &str) -> Result<(Vec<u8>, String), Error> {
    let supabase_url = get_env_var("SUPABASE_URL").unwrap();
    let api_key = get_env_var("SUPABASE_API_KEY").unwrap();
    let bucket_name = get_env_var("S3_BUCKET_NAME").unwrap();

    let http_client = reqwest::ClientBuilder::new()
        .timeout(std::time::Duration::from_secs(60))
        .tcp_keepalive(Some(std::time::Duration::from_secs(30)))
        .pool_max_idle_per_host(10)
        .build()
        .unwrap();

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

    if !file_response.status().is_success() {
        let status = file_response.status();
        let error_text = file_response.text().await.unwrap_or_default();
        println!("Error accessing file: {} - {}", status, error_text);
        return Err(anyhow!(
            "Failed to fetch bundle data: HTTP {}: {}",
            status,
            error_text
        ));
    }

    let content_type = file_response
        .headers()
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("application/octet-stream")
        .to_string();

    let bytes = file_response.bytes().await?.to_vec();

    Ok((bytes, content_type))
}
