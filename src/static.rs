use std::sync::LazyLock;

pub static INTERNAL_KEY: LazyLock<String> =
    LazyLock::new(|| std::env::var("BYPASS_INTERNAL_KEY").unwrap_or("".to_string()));
