use axum::{Router, routing::get, routing::post};

use std::time::Duration;
use tower_http::cors::{Any, CorsLayer};
use tower_http::limit::RequestBodyLimitLayer;
use tower_http::timeout::TimeoutLayer;

use crate::server::handlers::{
    download_object_handler, server_status_handler, upload_binary_handler,
};
use crate::server::types::AppState;
use crate::utils::constants::SERVER_REQUEST_BODY_LIMIT;
use crate::utils::get_env::get_env_var;
use reqwest::Client;
use shuttle_runtime::SecretStore;
use std::sync::Arc;

pub mod core;
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
    let state = Arc::new(app_state);
    println!("supabase connected to: {}", state.supabase_url);

    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    let timeout = TimeoutLayer::new(Duration::from_secs(3600));
    let request_body_limit = RequestBodyLimitLayer::new(SERVER_REQUEST_BODY_LIMIT);

    let router = Router::new()
        .route("/", get(server_status_handler))
        .route("/upload-binary", post(upload_binary_handler))
        .route("/download/{optimistic_hash}", get(download_object_handler))
        .layer(timeout)
        .layer(cors)
        .layer(request_body_limit)
        .with_state(state);

    Ok(router.into())
}
