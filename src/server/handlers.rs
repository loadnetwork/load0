use crate::orchestrator::db::insert_bundle;
use crate::server::types::{AppState, UploadQuery, UploadResponse};
use crate::utils::constants::ZERO_ADDRESS;
use crate::utils::hash::generate_pseudorandom_keccak_hash;
use axum::body::Body;
use axum::extract::Path;
use axum::response::IntoResponse;
use axum::{
    Json,
    extract::{Query, State},
    http::StatusCode,
};
use futures::StreamExt;
use futures::stream::{self, Stream};
use serde_json::{Value, json};
use std::convert::Infallible;
use std::sync::Arc;
use tokio_util::io::StreamReader;

// server status handler
pub async fn server_status_handler() -> Json<Value> {
    Json(json!({"status": "running"}))
}

// uploads handler
pub async fn upload_binary_handler(
    State(state): State<Arc<AppState>>,
    Query(params): Query<UploadQuery>,
    headers: axum::http::HeaderMap,
    body: axum::body::Body,
) -> impl IntoResponse {
    println!("UPLOAD BINARY HANDLER CALLED!");

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

    let stream = body.into_data_stream();

    let mut full_body = Vec::new();
    let mut stream =
        stream.map(|result| result.map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e)));
    let mut stream_reader = StreamReader::new(stream);

    match tokio::io::copy(&mut stream_reader, &mut full_body).await {
        Ok(_) => {
            println!("Total body size: {} bytes", full_body.len());
        }
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(UploadResponse {
                    success: false,
                    message: format!("Error reading request body: {}", e),
                    optimistic_hash: None,
                }),
            );
        }
    }

    // For Supabase REST API instead of S3 compatible API due to some weird API structure from supabase-s3 side
    let rest_url = state.supabase_url.replace("/v1/s3", "/v1/object");
    let url = format!("{}/{}/{}", rest_url, state.bucket_name, filename_hash);

    match state
        .http_client
        .post(&url)
        .header("Content-Type", &content_type)
        .header("Authorization", format!("Bearer {}", state.api_key))
        .header("apikey", &state.api_key)
        .body(full_body.clone())
        .send()
        .await
    {
        Ok(response) => {
            if response.status().is_success() {
                let _ = insert_bundle(&filename_hash, ZERO_ADDRESS, full_body.len() as u32, false)
                    .await
                    .unwrap();
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

// server handler to stream objects
pub async fn download_object_handler(
    State(state): State<Arc<AppState>>,
    Path(filename): Path<String>,
) -> impl IntoResponse {
    let direct_url = format!(
        "{}/object/public/{}/{}",
        state.supabase_url.replace("/v1/s3", "/v1"),
        state.bucket_name,
        filename
    );

    let file_response = match state
        .http_client
        .get(&direct_url)
        .header("apikey", &state.api_key)
        .header("Authorization", format!("Bearer {}", state.api_key))
        .send()
        .await
    {
        Ok(response) => response,
        Err(e) => {
            println!("Error requesting file: {}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to request file: {}", e),
            )
                .into_response();
        }
    };

    if !file_response.status().is_success() {
        let status = file_response.status();
        let error_text = file_response.text().await.unwrap_or_default();
        println!("Error accessing file: {} - {}", status, error_text);

        return (
            StatusCode::NOT_FOUND,
            format!("File not found: {}. Error: {}", filename, error_text),
        )
            .into_response();
    }
    let content_type = file_response
        .headers()
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("application/octet-stream")
        .to_string();

    let is_video = content_type.starts_with("video/");
    let bytes = match file_response.bytes().await {
        Ok(b) => b,
        Err(e) => {
            println!("Error reading file bytes: {}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to read file content: {}", e),
            )
                .into_response();
        }
    };

    let stream = stream::once(async move { Ok::<_, Infallible>(bytes) });

    let body = Body::from_stream(stream);

    if is_video {
        axum::response::Response::builder()
            .status(StatusCode::OK)
            .header("content-type", content_type)
            .header("accept-ranges", "bytes")
            .header("cache-control", "public, max-age=31536000")
            .body(body)
            .unwrap()
            .into_response()
    } else {
        axum::response::Response::builder()
            .status(StatusCode::OK)
            .header("content-type", content_type)
            .header("transfer-encoding", "chunked")
            .body(body)
            .unwrap()
            .into_response()
    }
}
