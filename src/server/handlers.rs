use crate::orchestrator::db::{
    get_bundle_by_optimistic_hash, get_bundle_by_txid, get_bundle_stats, insert_bundle,
};
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
use futures::stream::{self};
use serde_json::{Value, json};
use std::convert::Infallible;
use std::sync::Arc;
use tokio::io::AsyncReadExt;
use tokio_util::io::StreamReader;

// server status handler
pub async fn server_status_handler() -> Json<Value> {
    Json(json!({"status": "running"}))
}

// uploads handler with improved chunked reading and detailed logging
pub async fn upload_binary_handler(
    State(state): State<Arc<AppState>>,
    Query(params): Query<UploadQuery>,
    headers: axum::http::HeaderMap,
    body: axum::body::Body,
) -> impl IntoResponse {
    let start_time = std::time::Instant::now();
    println!("UPLOAD BINARY HANDLER CALLED");
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

    println!("CONTENT TYPE: {:?}", content_type);

    // let is_large_file = content_type.starts_with("video/") ||
    //                     content_type.starts_with("audio/") ||
    //                     content_type.starts_with("application/octet-stream") ||
    //                     content_type.starts_with("image/");

    let stream = body.into_data_stream();
    let mut full_body = Vec::new();
    let stream =
        stream.map(|result| result.map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e)));
    let mut stream_reader = StreamReader::new(stream);

    // read the data in chunks
    let read_start = std::time::Instant::now();
    let mut buffer = vec![0u8; 64 * 1024]; // 64KB buffer
    let mut total_bytes = 0;

    loop {
        match stream_reader.read(&mut buffer).await {
            Ok(0) => break,
            Ok(n) => {
                full_body.extend_from_slice(&buffer[..n]);
                total_bytes += n;

                if total_bytes % (5 * 1024 * 1024) < n {
                    // Log every ~5MB
                    println!(
                        "Read progress: {} MB in {:?}",
                        total_bytes / (1024 * 1024),
                        read_start.elapsed()
                    );
                }
            }
            Err(e) => {
                println!("Error reading request body: {}", e);
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(UploadResponse {
                        success: false,
                        message: format!("Error reading request body: {}", e),
                        optimistic_hash: None,
                    }),
                )
                    .into_response();
            }
        }
    }

    println!(
        "Total body size: {} bytes, read in {:?}",
        total_bytes,
        read_start.elapsed()
    );

    // For large files, log the size in MB
    if total_bytes > 5 * 1024 * 1024 {
        // If larger than 5MB
        println!("Large file upload: {} MB", total_bytes / (1024 * 1024));
    }

    let rest_url = state.supabase_url.replace("/v1/s3", "/v1/object");
    let url = format!("{}/{}/{}", rest_url, state.bucket_name, filename_hash);

    println!("Uploading to URL: {}", url);

    let upload_start = std::time::Instant::now();
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
            println!("Upload completed in {:?}", upload_start.elapsed());

            if response.status().is_success() {
                let db_start = std::time::Instant::now();
                match insert_bundle(
                    &filename_hash,
                    ZERO_ADDRESS,
                    full_body.len() as u32,
                    false,
                    &content_type,
                )
                .await
                {
                    Ok(_) => {
                        println!("Database record created in {:?}", db_start.elapsed());
                        println!("Total upload handler time: {:?}", start_time.elapsed());

                        (
                            StatusCode::OK,
                            Json(UploadResponse {
                                success: true,
                                message: format!(
                                    "Upload successful. Size: {} bytes, Time: {:?}",
                                    full_body.len(),
                                    start_time.elapsed()
                                ),
                                optimistic_hash: Some(filename_hash),
                            }),
                        )
                            .into_response()
                    }
                    Err(e) => {
                        println!("Error inserting bundle record: {:?}", e);

                        (
                            StatusCode::OK,
                            Json(UploadResponse {
                                success: true,
                                message: format!(
                                    "Upload successful but failed to create database record: {}",
                                    e
                                ),
                                optimistic_hash: Some(filename_hash),
                            }),
                        )
                            .into_response()
                    }
                }
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
                    .into_response()
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
                .into_response()
        }
    }
}

// server handler to stream objects
pub async fn download_object_handler(
    State(state): State<Arc<AppState>>,
    Path(filename): Path<String>,
) -> impl IntoResponse {
    let start_time = std::time::Instant::now();

    let object_metadata = match get_bundle_by_optimistic_hash(&filename).await {
        Ok(metadata) => {
            println!("REQUESTED BUNDLE: {:?}", metadata);
            metadata
        }
        Err(e) => {
            println!("Error getting bundle metadata: {}", e);
            return (
                StatusCode::NOT_FOUND,
                format!("Bundle not found: {}. Error: {}", filename, e),
            )
                .into_response();
        }
    };

    let content_type = object_metadata.content_type;
    println!("RENDERING MIME TYPE: {:?}", content_type);

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

    let bytes_start = std::time::Instant::now();
    let bytes = match file_response.bytes().await {
        Ok(b) => {
            println!(
                "Downloaded {} bytes in {:?}",
                b.len(),
                bytes_start.elapsed()
            );
            b
        }
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

    println!(
        "Download handler setup completed in {:?}",
        start_time.elapsed()
    );

    let is_video = content_type.starts_with("video/");

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
            .header("cache-control", "public, max-age=3600") // 1 hour cache for non-video content
            .body(body)
            .unwrap()
            .into_response()
    }
}

pub async fn get_bundle_by_op_hash_handler(Path(op_hash): Path<String>) -> Json<Value> {
    let bundle = get_bundle_by_optimistic_hash(&op_hash).await.unwrap();
    Json(serde_json::to_value(bundle).unwrap())
}

pub async fn get_bundle_by_load_txid_handler(Path(bundle_txid): Path<String>) -> Json<Value> {
    let bundle = get_bundle_by_txid(&bundle_txid).await.unwrap();
    Json(serde_json::to_value(bundle).unwrap())
}

pub async fn bundles_stats_handler() -> Json<Value> {
    let stats = get_bundle_stats().await.unwrap();
    Json(serde_json::to_value(stats).unwrap())
}
