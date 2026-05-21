use sha2::{Digest, Sha256};

pub fn hash_text(text: &str) -> String {
    let normalized = text.split_whitespace().collect::<Vec<_>>().join(" ");
    let mut h = Sha256::new();
    h.update(normalized.as_bytes());
    format!("{:x}", h.finalize())
}

pub fn hash_image_bytes(bytes: &[u8]) -> String {
    let mut h = Sha256::new();
    h.update(bytes);
    format!("{:x}", h.finalize())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hash_text_normalizes_whitespace() {
        let a = hash_text("  hello   world  ");
        let b = hash_text("hello world");
        assert_eq!(a, b);
    }

    #[test]
    fn hash_text_different_content_different_hash() {
        let a = hash_text("hello");
        let b = hash_text("world");
        assert_ne!(a, b);
    }

    #[test]
    fn hash_image_bytes_deterministic() {
        let bytes = b"fake png data";
        let a = hash_image_bytes(bytes);
        let b = hash_image_bytes(bytes);
        assert_eq!(a, b);
    }
}
