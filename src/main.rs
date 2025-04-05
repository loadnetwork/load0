use axum::{Router, routing::get, routing::post};

use std::time::Duration;
use tower_http::cors::{Any, CorsLayer};
use tower_http::limit::RequestBodyLimitLayer;
use tower_http::timeout::TimeoutLayer;

use crate::orchestrator::cron::update;
use crate::orchestrator::db::get_unsettled_bundles;
use crate::server::handlers::{
    bundles_stats_handler, download_object_handler, get_bundle_by_load_txid_handler,
    get_bundle_by_op_hash_handler, server_status_handler, upload_binary_handler,
};
use crate::server::types::AppState;
use crate::utils::constants::SERVER_REQUEST_BODY_LIMIT;
use crate::utils::get_env::get_env_var;
use reqwest::Client;
use shuttle_runtime::SecretStore;
use std::sync::Arc;

pub mod core;
pub mod orchestrator;
pub mod server;
pub mod utils;

// Initialize app state from secrets
async fn init_app_state(secrets: &SecretStore) -> Result<AppState, anyhow::Error> {
    let supabase_url = secrets
        .get("SUPABASE_URL")
        .unwrap_or_else(|| get_env_var("SUPABASE_URL").unwrap());

    let api_key = secrets
        .get("SUPABASE_API_KEY")
        .unwrap_or_else(|| get_env_var("SUPABASE_API_KEY").unwrap());

    let bucket_name = secrets
        .get("AWS_BUCKET_NAME")
        .unwrap_or_else(|| get_env_var("AWS_BUCKET_NAME").unwrap());

    // Create HTTP client
    let http_client = Client::new();

    Ok(AppState {
        http_client,
        supabase_url,
        bucket_name,
        api_key,
    })
}

#[shuttle_runtime::main]
async fn main(#[shuttle_runtime::Secrets] secrets: SecretStore) -> shuttle_axum::ShuttleAxum {
    let app_state = init_app_state(&secrets).await?;

    secrets.into_iter().for_each(|(key, val)| unsafe {
        std::env::set_var(key, val);
    });

    let state = Arc::new(app_state);
    // Spawn a background task for updates with backpressure control
    // tokio::spawn(async move {
    //     // Create a semaphore to limit concurrent operations
    //     let semaphore = Arc::new(tokio::sync::Semaphore::new(2)); // Limit to 2 concurrent operations

    //     loop {
    //         // Check if there are any unsettled bundles before acquiring a permit
    //         let unsettled_count = match get_unsettled_bundles().await {
    //             Ok(bundles) => bundles.len(),
    //             Err(_) => 0,
    //         };

    //         if unsettled_count == 0 {
    //             // No work to do, sleep longer
    //             println!("No unsettled bundles, sleeping for 120s");
    //             tokio::time::sleep(tokio::time::Duration::from_secs(120)).await;
    //             continue;
    //         }

    //         // Try to acquire a permit with timeout
    //         let permit = match tokio::time::timeout(
    //             Duration::from_secs(5),
    //             semaphore.clone().acquire_owned()
    //         ).await {
    //             Ok(Ok(permit)) => permit,
    //             Ok(Err(_)) => {
    //                 // Semaphore was closed
    //                 println!("Semaphore closed, retrying in 60s");
    //                 tokio::time::sleep(tokio::time::Duration::from_secs(60)).await;
    //                 continue;
    //             },
    //             Err(_) => {
    //                 // Timeout acquiring permit, system might be under load
    //                 println!("Timeout acquiring permit, system under load, sleeping for 60s");
    //                 tokio::time::sleep(tokio::time::Duration::from_secs(60)).await;
    //                 continue;
    //             }
    //         };

    //         // Process one bundle at a time with the permit
    //         tokio::spawn(async move {
    //             // The permit is moved into this task and will be released when the task completes
    //             let _permit = permit;

    //             // Process a single bundle
    //             if let Err(e) = update().await {
    //                 println!("Error in update: {:?}", e);
    //             }
    //         });

    //         // Add a small delay between spawns to prevent resource contention
    //         tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
    //     }
    // });
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    let timeout = TimeoutLayer::new(Duration::from_secs(3600));
    let request_body_limit = RequestBodyLimitLayer::new(SERVER_REQUEST_BODY_LIMIT);

    let router = Router::new()
        .route("/", get(server_status_handler))
        .route("/stats", get(bundles_stats_handler))
        .route("/upload", post(upload_binary_handler))
        .route("/download/{optimistic_hash}", get(download_object_handler))
        // to maintain same route as gateway.load.rs
        .route("/resolve/{optimistic_hash}", get(download_object_handler))
        .route(
            "/bundle/optimistic/{op_hash}",
            get(get_bundle_by_op_hash_handler),
        )
        .route(
            "/bundle/load/{bundle_txid}",
            get(get_bundle_by_load_txid_handler),
        )
        .layer(timeout)
        .layer(cors)
        .layer(request_body_limit)
        .with_state(state);

    Ok(router.into())
}
