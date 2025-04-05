use crate::utils::get_env::get_env_var;
use anyhow::Error;
use planetscale_driver::{Database, PSConnection, query};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Database)]
pub struct Bundle {
    pub id: u32,
    pub optimistic_hash: String,
    pub bundle_txid: String,
    pub data_size: u32,
    pub is_settled: bool,
    pub content_type: String,
}

#[derive(Debug, Serialize, Deserialize, Database)]
pub struct BundleOptimisticHash {
    pub optimistic_hash: String,
}

#[derive(Debug, Serialize, Deserialize, Database)]
pub struct BundleStats {
    pub bundles_count: u32,
    pub settled_count: u32,
    pub total_data_size: u128,
}

pub async fn ps_client() -> Result<PSConnection, Error> {
    let host = get_env_var("PS_DATABASE_HOST")?;
    let username = get_env_var("PS_DATABASE_USERNAME")?;
    let password = get_env_var("PS_DATABASE_PASSWORD")?;
    let conn: PSConnection = PSConnection::new(&host, &username, &password);
    Ok(conn)
}

pub async fn insert_bundle(
    optimistic_hash: &str,
    bundle_txid: &str,
    data_size: u32,
    is_settled: bool,
    content_type: &str,
) -> Result<(), Error> {
    let conn = ps_client().await?;
    let query_str = format!(
        "INSERT INTO bundles(optimistic_hash, bundle_txid, data_size, is_settled, content_type) VALUES(\"{}\", \"{}\", {}, {}, \"{}\")",
        optimistic_hash, bundle_txid, data_size, is_settled as u8, content_type
    );
    let res = query(&query_str).execute(&conn).await?;
    println!("Insert bundle operation successful: {:?}", res);
    Ok(())
}

pub async fn get_bundle_by_txid(bundle_txid: &str) -> Result<Bundle, Error> {
    let conn = ps_client().await?;
    let query_str = format!(
        "SELECT * FROM bundles WHERE bundle_txid = \"{}\"",
        bundle_txid
    );
    let result: Bundle = query(&query_str).fetch_one(&conn).await?;
    Ok(result)
}

pub async fn get_bundle_by_optimistic_hash(optimistic_hash: &str) -> Result<Bundle, Error> {
    let conn = ps_client().await?;
    let query_str = format!(
        "SELECT * FROM bundles WHERE optimistic_hash = \"{}\"",
        optimistic_hash
    );
    let result: Bundle = query(&query_str).fetch_one(&conn).await?;
    Ok(result)
}

pub async fn get_settled_bundles() -> Result<Vec<Bundle>, Error> {
    let conn = ps_client().await?;
    let query_str = "SELECT * FROM bundles WHERE is_settled = TRUE";
    let results: Vec<Bundle> = query(&query_str).fetch_all(&conn).await?;
    Ok(results)
}

pub async fn get_unsettled_bundles() -> Result<Vec<Bundle>, Error> {
    let conn = ps_client().await?;
    let query_str =
        "SELECT * FROM bundles WHERE is_settled = FALSE AND data_size > 0 ORDER BY id ASC LIMIT 5";

    let results: Vec<Bundle> = query(&query_str).fetch_all(&conn).await?;
    Ok(results)
}

pub async fn update_bundle_settled_status(
    optimistic_hash: &str,
    is_settled: bool,
    bundle_txid: &str,
) -> Result<(), Error> {
    let conn = ps_client().await?;
    let query_str = format!(
        "UPDATE bundles SET is_settled = {}, bundle_txid = \"{}\" WHERE optimistic_hash = \"{}\"",
        is_settled as u8, bundle_txid, optimistic_hash
    );
    let res = query(&query_str).execute(&conn).await?;
    println!(
        "Update bundle settled status and txid operation successful: {:?}",
        res
    );
    Ok(())
}

pub async fn update_bundle_content_type(
    optimistic_hash: &str,
    content_type: &str,
) -> Result<(), Error> {
    let conn = ps_client().await?;
    let query_str = format!(
        "UPDATE bundles SET content_type = \"{}\" WHERE optimistic_hash = \"{}\"",
        content_type, optimistic_hash
    );
    let res = query(&query_str).execute(&conn).await?;
    println!("Update bundle content_type operation successful: {:?}", res);
    Ok(())
}

pub async fn get_bundle_stats() -> Result<BundleStats, Error> {
    let conn = ps_client().await?;
    let query_str = "
        SELECT 
            COUNT(*) as bundles_count,
            SUM(CASE WHEN is_settled = TRUE THEN 1 ELSE 0 END) as settled_count,
            SUM(data_size) as total_data_size
        FROM bundles";
    let result: BundleStats = query(&query_str).fetch_one(&conn).await?;
    Ok(result)
}

pub async fn get_bundles_by_content_type(content_type: &str) -> Result<Vec<Bundle>, Error> {
    let conn = ps_client().await?;
    let query_str = format!(
        "SELECT * FROM bundles WHERE content_type = \"{}\"",
        content_type
    );
    let results: Vec<Bundle> = query(&query_str).fetch_all(&conn).await?;
    Ok(results)
}
