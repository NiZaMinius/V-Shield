use rand::rngs::OsRng;
/// Token generation and management for V-Shield
///
/// Tokens are 32 bytes of cryptographic keys for ChaCha20 encryption.
/// They are generated randomly, not derived from the file.
/// Each encryption produces a unique token.
use rand::RngCore;

/// Cryptographic token for ChaCha20-Poly1305 encryption
#[derive(Debug, Clone)]
pub struct Token {
    /// 32 bytes (256 bits) of random key material for ChaCha20
    pub key: [u8; 32],
    /// 16 bytes of video association (filled during first frame encoding)
    pub video_id: [u8; 16],
}

impl Token {
    /// Generate a new random token
    ///
    /// This is called EVERY time you encode a file, even if it's the same file.
    /// Each token is unique and independent of the file content.
    ///
    /// # Returns
    /// A new Token with random key and video_id
    pub fn generate() -> Self {
        let mut key = [0u8; 32];
        let mut video_id = [0u8; 16];

        OsRng.fill_bytes(&mut key);
        OsRng.fill_bytes(&mut video_id);

        Self { key, video_id }
    }

    /// Serialize token to a string for user display and storage
    ///
    /// Format: `vshield://<base58(key || video_id)>`
    ///
    /// Total 48 bytes (32 + 16) encoded as Base58
    /// Base58 encoding of 48 bytes ≈ 64 characters
    ///
    /// # Panics
    /// Never panics - uses bs58::encode which always succeeds
    pub fn to_string(&self) -> String {
        let mut raw = Vec::with_capacity(48);
        raw.extend_from_slice(&self.key);
        raw.extend_from_slice(&self.video_id);

        let encoded = bs58::encode(raw).into_string();
        format!("vshield://{}", encoded)
    }

    /// Parse token from a string representation
    ///
    /// # Arguments
    /// * `s` - Token string, format: `vshield://<base58>`
    ///
    /// # Returns
    /// * `Ok(Token)` if parsing succeeded
    /// * `Err(String)` if format is invalid or decoding fails
    ///
    /// # Errors
    /// Returns error if:
    /// - String doesn't start with "vshield://"
    /// - Base58 decoding fails
    /// - Decoded bytes != 48 bytes (32 key + 16 video_id)
    pub fn from_str(s: &str) -> Result<Self, String> {
        // Validate prefix
        let encoded = s
            .strip_prefix("vshield://")
            .ok_or("Токен должен начинаться с 'vshield://'")?;

        // Decode Base58
        let raw = bs58::decode(encoded)
            .into_vec()
            .map_err(|e| format!("Невалидная кодировка Base58: {}", e))?;

        // Verify length
        if raw.len() != 48 {
            return Err(format!(
                "Неверная длина токена: {} байт (ожидалось 48)",
                raw.len()
            ));
        }

        // Extract key and video_id
        let mut key = [0u8; 32];
        let mut video_id = [0u8; 16];
        key.copy_from_slice(&raw[..32]);
        video_id.copy_from_slice(&raw[32..48]);

        Ok(Self { key, video_id })
    }

    /// Get the encryption key (32 bytes for ChaCha20)
    #[inline]
    pub fn key_bytes(&self) -> &[u8; 32] {
        &self.key
    }

    /// Get the file/video association bytes
    #[inline]
    pub fn video_id_bytes(&self) -> &[u8; 16] {
        &self.video_id
    }

    /// Set the video_id (called during first frame processing)
    pub fn set_video_id(&mut self, video_id: [u8; 16]) {
        self.video_id = video_id;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const TOKEN_STRING_MAX: usize = 100; // 64 for Base58 + prefix vshield://

    #[test]
    fn token_generate() {
        let token = Token::generate();
        assert_eq!(token.key.len(), 32);
        assert_eq!(token.video_id.len(), 16);
    }

    #[test]
    fn token_uniqueness() {
        let t1 = Token::generate();
        let t2 = Token::generate();

        // Keys should be different (astronomically unlikely to be same)
        assert_ne!(t1.key, t2.key);
    }

    #[test]
    fn token_round_trip() {
        let original = Token::generate();
        let serialized = original.to_string();

        // Should start with prefix
        assert!(serialized.starts_with("vshield://"));

        // Should be valid length
        assert!(serialized.len() > 20);
        assert!(serialized.len() <= TOKEN_STRING_MAX);

        // Should deserialize correctly
        let restored = Token::from_str(&serialized).expect("Failed to parse token");

        assert_eq!(original.key, restored.key);
        assert_eq!(original.video_id, restored.video_id);
    }

    #[test]
    fn token_invalid_prefix() {
        let result = Token::from_str("invalid://abc123");
        assert!(result.is_err());
    }

    #[test]
    fn token_corrupted_data() {
        let token = Token::generate();
        let serialized = token.to_string();

        let corrupted = serialized + "AAAA";

        let result = Token::from_str(&corrupted);
        assert!(result.is_err());
    }

    #[test]
    fn token_same_file_different_tokens() {
        // Encoding the same file twice should produce different tokens
        // This is what we want - randomness, not determinism
        let token1 = Token::generate();
        let token2 = Token::generate();

        assert_ne!(token1.to_string(), token2.to_string());
    }
}
