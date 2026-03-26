use chacha20poly1305::{aead::Aead, ChaCha20Poly1305, Nonce};
use sha2::{Digest, Sha256};
use std::fs;
use std::path::Path;
/// V-Shield Decoder
///
/// Extracts hidden data from video frames that were encoded with the V-Shield Encoder.
///
/// Pipeline:
/// 1. Load frame(s) from video
/// 2. Detect finder patterns (anchors)
/// 3. Correct geometric distortions
/// 4. Extract color data from blocks
/// 5. Apply Reed-Solomon error correction
/// 6. De-interleave data
/// 7. Decrypt with token
/// 8. Decompress and output file
use vshield_core::{
    protocol::{Frame, MetadataBlock},
    VERSION,
};

pub struct DecoderConfig {
    pub input_frames_dir: String,
    pub output_file: String,
    pub token: String,
}

impl Default for DecoderConfig {
    fn default() -> Self {
        DecoderConfig {
            input_frames_dir: String::new(),
            output_file: String::new(),
            token: String::new(),
        }
    }
}

/// Main decoder structure
pub struct Decoder {
    config: DecoderConfig,
}

impl Decoder {
    pub fn new(config: DecoderConfig) -> Self {
        Decoder { config }
    }

    /// Decode frames into original file
    pub fn decode(&self) -> Result<DecodedOutput, Box<dyn std::error::Error>> {
        println!("[Decoder] V-Shield Decoder v{}", VERSION);
        println!(
            "[Decoder] Reading frames from: {}",
            self.config.input_frames_dir
        );

        // Load frames
        let frames = self.load_frames()?;
        println!("[Decoder] Loaded {} frames", frames.len());

        if frames.is_empty() {
            return Err("No frames found".into());
        }

        // Extract metadata from first frame
        let first_frame = &frames[0];
        let metadata = first_frame
            .metadata
            .clone()
            .ok_or("No metadata in first frame")?;

        println!("[Decoder] File: {}", metadata.filename);
        println!("[Decoder] Size: {} bytes", metadata.file_size);
        println!("[Decoder] Expected Hash: {:x?}", &metadata.file_hash[..16]);

        // Extract and concatenate data from all frames
        let mut all_data = Vec::new();
        for (idx, frame) in frames.iter().enumerate() {
            let frame_data = self.extract_frame_data(frame)?;
            let data_len = frame_data.len();
            all_data.extend(frame_data);
            println!("[Decoder] Extracted {} bytes from frame {}", data_len, idx);
        }

        println!("[Decoder] Total extracted data: {} bytes", all_data.len());

        // Apply error correction (Reed-Solomon decoding)
        let decoded_data = self.apply_error_correction(&all_data)?;
        println!(
            "[Decoder] After error correction: {} bytes",
            decoded_data.len()
        );

        // De-interleave data
        let de_interleaved = self.de_interleave_data(&decoded_data)?;
        println!(
            "[Decoder] After de-interleaving: {} bytes",
            de_interleaved.len()
        );

        // Decrypt data
        let plaintext = decrypt_data(&de_interleaved, &self.config.token)?;
        println!("[Decoder] Decrypted: {} bytes", plaintext.len());

        // Verify hash
        let mut hasher = Sha256::new();
        hasher.update(&plaintext);
        let computed_hash: [u8; 32] = hasher.finalize().into();

        if computed_hash != metadata.file_hash {
            eprintln!("[Decoder] ⚠️  Hash mismatch!");
            eprintln!("  Expected: {:x?}", &metadata.file_hash[..16]);
            eprintln!("  Got:      {:x?}", &computed_hash[..16]);
            return Err("Hash verification failed".into());
        }

        println!("[Decoder] ✓ Hash verified");

        Ok(DecodedOutput {
            data: plaintext,
            metadata,
            num_frames: frames.len() as u32,
        })
    }

    fn load_frames(&self) -> Result<Vec<Frame>, Box<dyn std::error::Error>> {
        // For now, load from metadata.json and frame PNGs
        let metadata_path = format!("{}/metadata.json", self.config.input_frames_dir);

        if !Path::new(&metadata_path).exists() {
            return Err("metadata.json not found".into());
        }

        let metadata_json = fs::read_to_string(&metadata_path)?;
        let metadata: serde_json::Value = serde_json::from_str(&metadata_json)?;

        let num_frames = metadata["num_frames"]
            .as_u64()
            .ok_or("num_frames not in metadata")? as usize;

        let mut frames = Vec::new();
        for idx in 0..num_frames {
            let frame_path = format!("{}/frame_{:04}.png", self.config.input_frames_dir, idx);

            if Path::new(&frame_path).exists() {
                // Load PNG and convert to Frame
                let frame = self.load_frame_from_png(&frame_path)?;
                frames.push(frame);
            }
        }

        Ok(frames)
    }

    fn load_frame_from_png(&self, path: &str) -> Result<Frame, Box<dyn std::error::Error>> {
        use image::open;

        let img_buf = open(path)?;
        let img = img_buf.to_rgb8();

        let width = img.width();
        let height = img.height();

        // Convert image to pixel data
        let mut pixel_data = Vec::new();
        for pixel in img.pixels() {
            pixel_data.push(pixel[0]); // R
            pixel_data.push(pixel[1]); // G
            pixel_data.push(pixel[2]); // B
        }

        // Create frame (frame_id will be determined from path or header)
        let mut frame = Frame::new(0, 8, width, height, false);
        frame.pixel_data = Some(pixel_data);

        Ok(frame)
    }

    fn extract_frame_data(&self, _frame: &Frame) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
        // Extract color values from blocks and convert to bytes
        // TODO: Implement actual extraction from pixel data
        Ok(Vec::new())
    }

    fn apply_error_correction(&self, data: &[u8]) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
        // Apply Reed-Solomon decoding
        // For now, return as-is (actual implementation would decode ECC)
        Ok(data.to_vec())
    }

    fn de_interleave_data(&self, data: &[u8]) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
        // Apply de-interleaving (reverse of encoding)
        // TODO: Implement actual de-interleaving using InterleavingMap
        Ok(data.to_vec())
    }
}

/// Output of decoding
pub struct DecodedOutput {
    pub data: Vec<u8>,
    pub metadata: MetadataBlock,
    pub num_frames: u32,
}

impl DecodedOutput {
    pub fn save(&self, output_path: &str) -> Result<(), Box<dyn std::error::Error>> {
        fs::write(output_path, &self.data)?;
        println!("[Decoder] File saved to {}", output_path);
        Ok(())
    }
}

/// Decrypt data using ChaCha20-Poly1305
fn decrypt_data(ciphertext: &[u8], token: &str) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    use chacha20poly1305::aead::KeyInit;

    // Derive a 32-byte key from the token (same as encoder)
    let mut hasher = Sha256::new();
    hasher.update(token.as_bytes());
    let key_bytes: [u8; 32] = hasher.finalize().into();

    let cipher = ChaCha20Poly1305::new(key_bytes[..].into());

    // Use the same fixed nonce (must match encoder's nonce)
    let nonce = Nonce::from([0u8; 12]);

    let plaintext = cipher
        .decrypt(&nonce, ciphertext)
        .map_err(|e| format!("Decryption failed: {}", e))?;

    Ok(plaintext)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_decryption() {
        let token = "test-token";
        let plaintext = b"Hello, V-Shield!";

        // Encrypt
        let ciphertext = {
            let mut hasher = Sha256::new();
            hasher.update(token.as_bytes());
            let key_bytes: [u8; 32] = hasher.finalize().into();
            let cipher = ChaCha20Poly1305::new(Key::<ChaCha20Poly1305>::from(key_bytes));
            let nonce = Nonce::from([0u8; 12]);
            cipher
                .encrypt(&nonce, plaintext)
                .expect("Encryption failed")
        };

        // Decrypt
        let decrypted = decrypt_data(&ciphertext, token).expect("Decryption failed");

        assert_eq!(plaintext, decrypted.as_slice());
    }
}
