use axum::{Router, routing::get, routing::post};
use std::net::SocketAddr;
use std::time::Duration;
use tower_http::cors::{Any, CorsLayer};
use tower_http::limit::RequestBodyLimitLayer;
use tower_http::timeout::TimeoutLayer;

use crate::booter::Booter;
use crate::governor_conf::get_governor_conf;
use crate::orchestrator::cron::update;
use crate::orchestrator::db::get_unsettled_bundles;
use crate::server::handlers::{
    bundles_stats_handler, download_object_handler, get_bundle_by_load_txid_handler,
    get_bundle_by_op_hash_handler, server_status_handler, upload_binary_handler,
};
use crate::server::rate_limiter::{
    LOAD_HEADER_NAME, XLoadAuthHeaderExtractor, is_whitelisted, whitelisted_urls,
};
use crate::server::types::AppState;
use crate::r#static::INTERNAL_KEY;
use crate::utils::auth::is_access_token_valid;
use crate::utils::constants::SERVER_REQUEST_BODY_LIMIT;
use crate::utils::get_env::get_env_var;
use axum::handler::HandlerWithoutStateExt;
use axum::http::Request;
use reqwest::Client;
use std::sync::Arc;
use tower::ServiceExt;
use tower_governor::GovernorLayer;
use tower_governor::governor::GovernorConfigBuilder;
use url::Url;

mod booter;
pub mod core;
mod governor_conf;
pub mod orchestrator;
pub mod server;
mod r#static;
pub mod utils;

// Initialize app state from environment variables
async fn init_app_state() -> Result<AppState, anyhow::Error> {
    let supabase_url = get_env_var("SUPABASE_URL").unwrap();
    let api_key = get_env_var("SUPABASE_API_KEY").unwrap();
    let bucket_name = get_env_var("S3_BUCKET_NAME").unwrap();

    // Create HTTP client
    let http_client = Client::new();

    Ok(AppState {
        http_client,
        supabase_url,
        bucket_name,
        api_key,
    })
}

fn get_load_burst_size() -> u32 {
    std::env::var("LOAD_BURST_SIZE")
        .ok()
        .and_then(|val| val.parse::<u32>().ok())
        .unwrap_or(5)
}

fn retrieval_routes() -> Router<Arc<AppState>> {
    Router::new()
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
}

fn retrieval_route_with_burst(burst_size_per_min: u32) -> Router<Arc<AppState>> {
    retrieval_routes().layer(GovernorLayer {
        config: Arc::new(get_governor_conf(burst_size_per_min)),
    })
}

fn get_router(state: Arc<AppState>) -> Router {
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    let timeout = TimeoutLayer::new(Duration::from_secs(3600));
    let request_body_limit = RequestBodyLimitLayer::new(SERVER_REQUEST_BODY_LIMIT);

    let unprotected_router = retrieval_route_with_burst(6);

    let protected_router = retrieval_route_with_burst(60);

    let internal_router = retrieval_route_with_burst(999999);

    let dispatch_state = state.clone();

    let whitelisted_urls = Arc::new(whitelisted_urls());

    let dispatch = tower::service_fn(move |req: Request<axum::body::Body>| {
        let internal_key = &*INTERNAL_KEY;
        let headers = req.headers();
        let whitelisted_urls = whitelisted_urls.clone();

        let req_header = headers.get(LOAD_HEADER_NAME);
        let host_header = headers
            .get("host")
            .map(|e| e.to_str().unwrap_or(""))
            .unwrap_or("");
        let host_url = {
            if host_header.starts_with("http://") || host_header.starts_with("https://") {
                host_header.to_string()
            } else {
                format!("http://{}", host_header)
            }
        };

        let is_url_whitelisted = is_whitelisted(Some(host_url), &whitelisted_urls);

        let tier_router = {
            if is_url_whitelisted {
                retrieval_routes()
            } else {
                match req_header.and_then(|h| h.to_str().ok()) {
                    Some(value) if value == internal_key => internal_router.clone(),
                    Some(value) if is_access_token_valid(value) => protected_router.clone(),
                    _ => unprotected_router.clone(),
                }
            }
        };

        let tier_router = tier_router.with_state(dispatch_state.clone());

        // forward the *same* request into the chosen sub‑router
        async move { tier_router.oneshot(req).await }
    });

    let router = Router::new()
        .route("/", get(server_status_handler))
        .route("/stats", get(bundles_stats_handler))
        .route("/upload", post(upload_binary_handler))
        .fallback_service(dispatch)
        .layer(timeout)
        .layer(cors)
        .layer(request_body_limit)
        .with_state(state);

    router
}

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    // Initialize tracing for logging
    tracing_subscriber::fmt::init();

    let app_state = init_app_state().await?;

    let state = Arc::new(app_state);
    // Spawn a background task for updates with backpressure control
    tokio::spawn(async move {
        // Create a semaphore to limit concurrent operations
        let semaphore = Arc::new(tokio::sync::Semaphore::new(2)); // Limit to 2 concurrent operations

        // loop {
        //     let unsettled_count = match get_unsettled_bundles().await {
        //         Ok(bundles) => bundles.len(),
        //         Err(_) => 0,
        //     };

        //     if unsettled_count == 0 {
        //         println!("No unsettled bundles, sleeping for 120s");
        //         tokio::time::sleep(tokio::time::Duration::from_secs(120)).await;
        //         continue;
        //     }

        //     let permit = match tokio::time::timeout(
        //         Duration::from_secs(5),
        //         semaphore.clone().acquire_owned(),
        //     )
        //     .await
        //     {
        //         Ok(Ok(permit)) => permit,
        //         Ok(Err(_)) => {
        //             println!("Semaphore closed, retrying in 60s");
        //             tokio::time::sleep(tokio::time::Duration::from_secs(60)).await;
        //             continue;
        //         }
        //         Err(_) => {
        //             println!("Timeout acquiring permit, system under load, sleeping for 60s");
        //             tokio::time::sleep(tokio::time::Duration::from_secs(60)).await;
        //             continue;
        //         }
        //     };

        //     tokio::spawn(async move {
        //         let _permit = permit;

        //         if let Err(e) = update().await {
        //             println!("Error in update: {:?}", e);
        //         }
        //     });

        //     tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
        // }
    });

    let booter = Booter::new(None).await;
    let _ = booter.start(get_router(state)).await;

    Ok(())
}
