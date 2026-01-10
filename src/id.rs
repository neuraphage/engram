//! ID generation for Engram items.

use chrono::{DateTime, Utc};
use rand::Rng;
use sha2::{Digest, Sha256};

/// Generate a unique ID from content + entropy.
/// Format: "eg-" + 10 hex chars of SHA256(title + timestamp + random)
pub fn generate_id(title: &str, created_at: DateTime<Utc>) -> String {
    let mut hasher = Sha256::new();
    hasher.update(title.as_bytes());
    hasher.update(created_at.timestamp_nanos_opt().unwrap_or(0).to_le_bytes());
    // Add 8 bytes of randomness to prevent collisions
    hasher.update(rand::rng().random::<[u8; 8]>());
    let hash = hasher.finalize();
    // 10 hex chars = 40 bits = ~1 trillion values
    format!(
        "eg-{:010x}",
        u64::from_be_bytes([hash[0], hash[1], hash[2], hash[3], hash[4], 0, 0, 0]) >> 24
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_id_format() {
        let id = generate_id("Test title", Utc::now());
        assert!(id.starts_with("eg-"));
        assert_eq!(id.len(), 13); // "eg-" + 10 hex chars
    }

    #[test]
    fn test_generate_id_uniqueness() {
        let now = Utc::now();
        let id1 = generate_id("Same title", now);
        let id2 = generate_id("Same title", now);
        // Due to random component, same inputs should produce different IDs
        assert_ne!(id1, id2);
    }

    #[test]
    fn test_generate_id_different_titles() {
        let now = Utc::now();
        let id1 = generate_id("Title one", now);
        let id2 = generate_id("Title two", now);
        assert_ne!(id1, id2);
    }
}
