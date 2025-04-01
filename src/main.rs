use axum::{
    Json, Router,
    extract::{Query, State},
    http::StatusCode,
    routing::get,
    routing::post,
};

use crate::server::handlers::{server_status_handler, upload_binary_handler};
use crate::server::types::AppState;
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

    let router = Router::new()
        .route("/", get(server_status_handler))
        .route("/upload-binary", post(upload_binary_handler))
        .with_state(state);

    Ok(router.into())
}
