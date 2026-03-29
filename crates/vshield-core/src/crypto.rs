/// Cryptographic functions for V-Shield
///
/// Uses ChaCha20-Poly1305 AEAD cipher for authenticated encryption.
/// Nonce is RANDOM and prepended to ciphertext (never reused with same key).
use chacha20poly1305::{
    aead::{Aead, KeyInit},
    ChaCha20Poly1305, Nonce,
};
use rand::RngCore;
use sha2::{Digest, Sha256};

/// Size of the authentication tag (Poly1305)
pub const TAG_SIZE: usize = 16;

/// Size of the nonce (ChaCha20)
pub const NONCE_SIZE: usize = 12;

/// Total overhead from encryption: nonce (12) + auth tag (16)
pub const ENCRYPTION_OVERHEAD: usize = NONCE_SIZE + TAG_SIZE;

/// Encrypt data using ChaCha20-Poly1305 with a random nonce
///
/// # Arguments
/// * `key` - 32-byte encryption key
/// * `plaintext` - Data to encrypt
///
/// # Returns
/// Ciphertext with format: [12-byte random nonce][encrypted data][16-byte auth tag]
///
/// The nonce is generated randomly and is NEVER reused with the same key.
/// This is essential for security - reuse would allow "two-time pad" attacks.
///
/// # Example
/// ```
/// use vshield_core::crypto::encrypt;
/// let key = [0u8; 32];
/// let plaintext = b"Hello, World!";
/// let encrypted = encrypt(&key, plaintext).unwrap;
/// assert!(encrypted.len() > plaintext.len()); // overhead for nonce + tag
/// ```
pub fn encrypt(key: &[u8; 32], plaintext: &[u8]) -> Result<Vec<u8>, String> {
    let cipher = ChaCha20Poly1305::new(key.into());

    // Generate a NEW random nonce for this encryption
    // This is critical - NEVER use the same (key, nonce) pair twice
    let mut nonce_bytes = [0u8; NONCE_SIZE];
    let mut rng = rand::rngs::OsRng;
    rng.fill_bytes(&mut nonce_bytes);
    let nonce = Nonce::from(nonce_bytes);

    // Encrypt with authentication
    let ciphertext = cipher
        .encrypt(&nonce, plaintext)
        .map_err(|e| format!("Шифрование не удалось: {}", e))?;

    // Format: [nonce (12 bytes)][ciphertext + auth tag]
    // This allows the decryptor to extract the nonce and decrypt
    let mut result = Vec::with_capacity(NONCE_SIZE + ciphertext.len());
    result.extend_from_slice(&nonce_bytes);
    result.extend_from_slice(&ciphertext);

    Ok(result)
}

/// Decrypt data encrypted with ChaCha20-Poly1305
///
/// # Arguments
/// * `key` - 32-byte decryption key
/// * `encrypted_data` - Data from encrypt(), format: [nonce][ciphertext][tag]
///
/// # Returns
/// * `Ok(plaintext)` on success
/// * `Err(message)` if:
///   - Data is too short (< 12 bytes for nonce)
///   - Decryption fails (wrong key or corrupted data)
///
/// # Example
/// ```
/// use vshield_core::crypto::decrypt;
/// use  use vshield_core::crypto::encrypt;
/// let key = [42u8; 32];
/// let plaintext = b"Secret message";
/// let encrypted = encrypt(&key, plaintext).unwrap();
/// let decrypted = decrypt(&key, &encrypted).unwrap();
/// assert_eq!(plaintext, decrypted.as_slice());
/// ```
pub fn decrypt(key: &[u8; 32], encrypted_data: &[u8]) -> Result<Vec<u8>, String> {
    // Validate minimum length
    if encrypted_data.len() < NONCE_SIZE {
        return Err(format!(
            "Данные слишком короткие - нет nonce ({} < {})",
            encrypted_data.len(),
            NONCE_SIZE
        ));
    }

    // Extract nonce from first 12 bytes
    let (nonce_bytes, ciphertext) = encrypted_data.split_at(NONCE_SIZE);
    let nonce = Nonce::from_slice(nonce_bytes);

    // Decrypt
    let cipher = ChaCha20Poly1305::new(key.into());
    cipher.decrypt(nonce, ciphertext).map_err(|_| {
        "Дешифрование не удалось - возможно неверный токен или повреждённые данные".to_string()
    })
}

/// Compute SHA-256 hash of data
///
/// # Arguments
/// * `data` - Data to hash
///
/// # Returns
/// 32-byte SHA-256 hash
pub fn hash_sha256(data: &[u8]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(data);
    let result = hasher.finalize();

    let mut hash = [0u8; 32];
    hash.copy_from_slice(&result);
    hash
}

/// Verify SHA-256 hash
///
/// # Arguments
/// * `data` - Data to verify
/// * `expected_hash` - Expected hash value
///
/// # Returns
/// `true` if hash matches, `false` otherwise
pub fn verify_sha256(data: &[u8], expected_hash: &[u8; 32]) -> bool {
    &hash_sha256(data) == expected_hash
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encrypt_decrypt_round_trip() {
        let key = [42u8; 32];
        let plaintext = b"Hello, V-Shield!";

        let encrypted = encrypt(&key, plaintext).expect("Encryption failed");
        let decrypted = decrypt(&key, &encrypted).expect("Decryption failed");

        assert_eq!(plaintext.to_vec(), decrypted);
    }

    #[test]
    fn encrypted_has_nonce() {
        let key = [0u8; 32];
        let plaintext = b"Test";

        let encrypted = encrypt(&key, plaintext).expect("Encryption failed");

        // Must have at least nonce (12) + tag (16) overhead
        assert!(encrypted.len() >= plaintext.len() + ENCRYPTION_OVERHEAD);

        // First 12 bytes should be (mostly) non-zero (random nonce)
        let nonce = &encrypted[..NONCE_SIZE];
        let is_all_zero = nonce.iter().all(|&b| b == 0);
        assert!(!is_all_zero, "Nonce should be random, not all zeros");
    }

    #[test]
    fn different_nonces_each_time() {
        let key = [7u8; 32];
        let plaintext = b"Same data";

        let enc1 = encrypt(&key, plaintext).expect("Enc1 failed");
        let enc2 = encrypt(&key, plaintext).expect("Enc2 failed");

        // Different nonces → different ciphertexts (even for same plaintext)
        assert_ne!(
            enc1, enc2,
            "Same plaintext should have different ciphertexts (different nonces)"
        );

        // Both should decrypt to same plaintext
        assert_eq!(decrypt(&key, &enc1).unwrap(), decrypt(&key, &enc2).unwrap());
    }

    #[test]
    fn wrong_key_fails() {
        let key1 = [1u8; 32];
        let key2 = [2u8; 32];
        let plaintext = b"Secret";

        let encrypted = encrypt(&key1, plaintext).expect("Encryption failed");
        let result = decrypt(&key2, &encrypted);

        // Decryption with wrong key should fail
        assert!(result.is_err());
    }

    #[test]
    fn corrupted_data_fails() {
        let key = [3u8; 32];
        let plaintext = b"This is secret";

        let mut encrypted = encrypt(&key, plaintext).expect("Encryption failed");

        // Flip a bit in the ciphertext (after nonce)
        encrypted[NONCE_SIZE] = encrypted[NONCE_SIZE].wrapping_add(1);

        // Should fail due to auth tag mismatch
        assert!(decrypt(&key, &encrypted).is_err());
    }

    #[test]
    fn too_short_data_fails() {
        let key = [0u8; 32];

        // Data shorter than nonce size
        let result = decrypt(&key, &[1, 2, 3]);
        assert!(result.is_err());
    }

    #[test]
    fn sha256_hash() {
        let data = b"Test data";
        let hash1 = hash_sha256(data);
        let hash2 = hash_sha256(data);

        // Same data should hash to same value
        assert_eq!(hash1, hash2);

        // Hash should be 32 bytes
        assert_eq!(hash1.len(), 32);
    }

    #[test]
    fn sha256_different_data() {
        let hash1 = hash_sha256(b"Data 1");
        let hash2 = hash_sha256(b"Data 2");

        // Different data should have different hashes
        assert_ne!(hash1, hash2);
    }

    #[test]
    fn verify_sha256_correct() {
        let data = b"V-Shield test";
        let hash = hash_sha256(data);

        assert!(verify_sha256(data, &hash));
    }

    #[test]
    fn verify_sha256_incorrect() {
        let data = b"Original";
        let wrong_data = b"Modified";
        let hash = hash_sha256(data);

        assert!(!verify_sha256(wrong_data, &hash));
    }

    #[test]
    fn empty_data() {
        let key = [0u8; 32];
        let plaintext = b"";

        let encrypted = encrypt(&key, plaintext).expect("Encryption of empty failed");
        let decrypted = decrypt(&key, &encrypted).expect("Decryption failed");

        assert!(decrypted.is_empty());
    }

    #[test]
    fn large_data() {
        let key = [11u8; 32];
        let plaintext = vec![42u8; 10000];

        let encrypted = encrypt(&key, &plaintext).expect("Encryption failed");
        let decrypted = decrypt(&key, &encrypted).expect("Decryption failed");

        assert_eq!(plaintext, decrypted);
    }
}
