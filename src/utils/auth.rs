pub const AUTH_API_URL: &str = "https://k8s.load-auth-service.load.network";

pub fn is_access_token_valid(token: &str) -> bool {
    let req = ureq::get(format!("{}/internal/verify/{}", AUTH_API_URL, token)).call();
    if let Ok(req) = req {
        req.status().is_success()
    } else {
        false
    }
}
