use sha3::{Digest, Keccak256};
use std::time::{SystemTime, UNIX_EPOCH};

pub fn generate_pseudorandom_keccak_hash() -> String {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let mut hasher = Keccak256::new();
    hasher.update(timestamp.to_string().as_bytes());

    let hash_result = hasher.finalize();
    let hash_hex = hex::encode(hash_result);

    format!("0x{}", hash_hex)
}
