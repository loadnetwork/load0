use reqwest::Client;
use serde::{Deserialize, Serialize};

#[derive(Clone)]
pub struct AppState {
    pub http_client: Client,
    pub supabase_url: String,
    pub bucket_name: String,
    pub api_key: String,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct UploadQuery {
    pub content_type: Option<String>,
}

// Response structure
#[derive(Serialize, Debug, Deserialize, Clone)]
pub struct UploadResponse {
    pub success: bool,
    pub message: String,
    pub optimistic_hash: Option<String>,
}
