use axum::{
    Json, Router,
    extract::{Query, State},
    http::StatusCode,
    routing::post,
};
use serde_json::{Value, json};

use crate::server::types::{AppState, UploadQuery, UploadResponse};
use crate::utils::hash::generate_pseudorandom_keccak_hash;
use bytes::Bytes;
use std::sync::Arc;

pub async fn server_status_handler() -> Json<Value> {
    Json(json!({"status": "running"}))
}

// Handler for binary uploads
pub async fn upload_binary_handler(
    State(state): State<Arc<AppState>>,
    Query(params): Query<UploadQuery>,
    headers: axum::http::HeaderMap,
    body: Bytes,
) -> (StatusCode, Json<UploadResponse>) {
    let filename_hash = generate_pseudorandom_keccak_hash();
    let content_type = params
        .content_type
        .or_else(|| {
            headers
                .get(axum::http::header::CONTENT_TYPE)
                .and_then(|h| h.to_str().ok())
                .map(String::from)
        })
        .unwrap_or_else(|| "application/octet-stream".to_string());

    println!("CONTENT TYPE {:?}", content_type);

    // For Supabase REST API instead of S3 compatible API due to some weird API structure from supabase-s3 side
    let rest_url = state.supabase_url.replace("/v1/s3", "/v1/object");
    let url = format!("{}/{}/{}", rest_url, state.bucket_name, filename_hash);

    match state
        .http_client
        .post(&url)
        .header("Content-Type", &content_type)
        .header("Authorization", format!("Bearer {}", state.api_key))
        .header("apikey", &state.api_key)
        .body(body)
        .send()
        .await
    {
        Ok(response) => {
            if response.status().is_success() {
                (
                    StatusCode::OK,
                    Json(UploadResponse {
                        success: true,
                        message: "Upload successful".to_string(),
                        optimistic_hash: Some(filename_hash),
                    }),
                )
            } else {
                let status = response.status();
                let error_text = response.text().await.unwrap_or_default();
                println!("Error: {} - {}", status, error_text);

                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(UploadResponse {
                        success: false,
                        message: format!("Upload failed: {} - {}", status, error_text),
                        optimistic_hash: None,
                    }),
                )
            }
        }
        Err(err) => {
            println!("Request error: {:?}", err);

            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(UploadResponse {
                    success: false,
                    message: format!("Upload failed: {}", err),
                    optimistic_hash: None,
                }),
            )
        }
    }
}
