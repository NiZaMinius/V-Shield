use chacha20poly1305::{aead::Aead, ChaCha20Poly1305, Nonce};
use sha2::{Digest, Sha256};
use std::fs;
use std::path::Path;
use uuid::Uuid;
/// V-Shield Encoder
///
/// Converts files into video frames that survive YouTube compression.
///
/// Pipeline:
/// 1. Read input file
/// 2. Compress (optional)
/// 3. Generate token for this file
/// 4. Encrypt data
/// 5. Apply Reed-Solomon ECC
/// 6. Interleave across frame
/// 7. Render to image/video
use vshield_core::{
    ecc::{ECCConfig, RSEncoder},
    interleave::{InterleavingMap, InterleavingStrategy},
    protocol::{Frame, MetadataBlock},
    VERSION,
};

const FRAME_WIDTH: u32 = 1920;
const FRAME_HEIGHT: u32 = 1080;
const BLOCK_SIZE: u8 = 8;

pub struct EncoderConfig {
    pub input_file: String,
    pub output_file: String,
    pub block_size: u8,
    pub redundancy_percent: u8,
    pub frame_width: u32,
    pub frame_height: u32,
}

impl Default for EncoderConfig {
    fn default() -> Self {
        EncoderConfig {
            input_file: String::new(),
            output_file: String::new(),
            block_size: BLOCK_SIZE,
            redundancy_percent: 25,
            frame_width: FRAME_WIDTH,
            frame_height: FRAME_HEIGHT,
        }
    }
}

/// Main encoder structure
pub struct Encoder {
    config: EncoderConfig,
}

impl Encoder {
    pub fn new(config: EncoderConfig) -> Self {
        Encoder { config }
    }

    /// Encode a file into video frames
    pub fn encode(&self) -> Result<EncodedOutput, Box<dyn std::error::Error>> {
        println!("[Encoder] V-Shield Encoder v{}", VERSION);
        println!("[Encoder] Reading input file: {}", self.config.input_file);

        // Read input file
        let file_data = fs::read(&self.config.input_file)?;
        let file_size = file_data.len();
        let filename = Path::new(&self.config.input_file)
            .file_name()
            .unwrap()
            .to_string_lossy()
            .to_string();

        println!("[Encoder] File size: {} bytes", file_size);

        // Calculate SHA-256 hash
        let mut hasher = Sha256::new();
        hasher.update(&file_data);
        let file_hash: [u8; 32] = hasher.finalize().into();
        let hash_hex = format!("{:02x?}", file_hash);
        println!(
            "[Encoder] SHA-256: {}...",
            &hash_hex[0..16.min(hash_hex.len())]
        );

        // Generate token (random key)
        let token = generate_token();
        println!("[Encoder] Generated token: {}", token);

        // Encrypt data
        let encrypted_data = encrypt_data(&file_data, &token)?;
        println!(
            "[Encoder] Encrypted data size: {} bytes",
            encrypted_data.len()
        );

        // Create Reed-Solomon configuration
        // RS requires total_symbols <= 255
        // Calculate safe redundancy based on data size
        let redundancy = if encrypted_data.len() > 150 {
            5 // Large files: minimal redundancy
        } else if encrypted_data.len() > 100 {
            10 // Medium files
        } else {
            self.config.redundancy_percent // Small files: use requested redundancy
        };

        let ecc_config = ECCConfig::new(encrypted_data.len(), redundancy);
        let rs_encoder = RSEncoder::new(ecc_config)?;

        // Encode with ECC
        let ecc_encoded = rs_encoder.encode(&encrypted_data)?;
        println!(
            "[Encoder] After ECC: {} bytes (applied {}% redundancy)",
            ecc_encoded.len(),
            redundancy
        );

        let bits_per_block = 3; // 8 colors = 3 bits per block (2^3)
        let bytes_per_frame = (self.config.frame_width * self.config.frame_height
            / (self.config.block_size as u32).pow(2)) as usize
            / bits_per_block.max(1);
        let num_frames = (ecc_encoded.len() + bytes_per_frame - 1) / bytes_per_frame;

        println!("[Encoder] Bytes per frame: {}", bytes_per_frame);
        println!("[Encoder] Total frames needed: {}", num_frames);

        // Create metadata for first frame
        let metadata = MetadataBlock {
            filename: filename.clone(),
            file_size: file_size as u64,
            file_hash,
            token_id: token.clone(),
        };

        // Generate frames
        let mut frames = Vec::new();
        for frame_id in 0..num_frames as u32 {
            let is_first = frame_id == 0;
            let frame = self.create_frame(
                frame_id,
                is_first,
                &ecc_encoded,
                bytes_per_frame,
                if is_first {
                    Some(metadata.clone())
                } else {
                    None
                },
            )?;
            frames.push(frame);
        }

        Ok(EncodedOutput {
            frames,
            token,
            metadata,
            num_frames: num_frames as u32,
        })
    }

    fn create_frame(
        &self,
        frame_id: u32,
        is_first: bool,
        data: &[u8],
        bytes_per_frame: usize,
        metadata: Option<MetadataBlock>,
    ) -> Result<Frame, Box<dyn std::error::Error>> {
        let mut frame = Frame::new(
            frame_id,
            self.config.block_size,
            self.config.frame_width,
            self.config.frame_height,
            is_first,
        );

        frame.metadata = metadata.clone();

        // Extract data for this frame
        let start_byte = (frame_id as usize) * bytes_per_frame;
        let end_byte = std::cmp::min(start_byte + bytes_per_frame, data.len());

        if start_byte >= data.len() {
            return Ok(frame); // Frame is beyond data (padding)
        }

        let frame_data = &data[start_byte..end_byte];

        // Create pixel buffer for frame
        let mut pixels =
            vec![0u8; (self.config.frame_width * self.config.frame_height * 3) as usize];

        // Draw finder patterns (anchors)
        self.draw_anchors(&mut pixels)?;

        // Draw frame header
        self.draw_header(&mut pixels, &frame)?;

        // Draw metadata (if first frame)
        if is_first && metadata.is_some() {
            self.draw_metadata(&mut pixels, &metadata.unwrap())?;
        }

        // Create interleaving map
        let total_blocks = (self.config.frame_width as usize / self.config.block_size as usize)
            * (self.config.frame_height as usize / self.config.block_size as usize);
        let interleaving_map = InterleavingMap::new(
            self.config.frame_width as usize / self.config.block_size as usize,
            self.config.frame_height as usize / self.config.block_size as usize,
            total_blocks,
            InterleavingStrategy::Pseudorandom { seed: 42 },
        );

        // Encode data into frame pixels
        self.encode_payload(&mut pixels, frame_data, &interleaving_map)?;

        frame.pixel_data = Some(pixels);
        Ok(frame)
    }

    fn draw_anchors(&self, pixels: &mut [u8]) -> Result<(), Box<dyn std::error::Error>> {
        // Draw 4 finder patterns in corners (like QR codes)
        // Each anchor is a 7x7 pattern: 1-1-3-1-1 ratio for scale invariance
        const ANCHOR_SIZE: u32 = 28; // 7 units * 4 pixels each

        // Top-left
        self.draw_anchor(pixels, 0, 0, ANCHOR_SIZE)?;
        // Top-right
        self.draw_anchor(
            pixels,
            self.config.frame_width - ANCHOR_SIZE,
            0,
            ANCHOR_SIZE,
        )?;
        // Bottom-left
        self.draw_anchor(
            pixels,
            0,
            self.config.frame_height - ANCHOR_SIZE,
            ANCHOR_SIZE,
        )?;
        // Bottom-right
        self.draw_anchor(
            pixels,
            self.config.frame_width - ANCHOR_SIZE,
            self.config.frame_height - ANCHOR_SIZE,
            ANCHOR_SIZE,
        )?;

        Ok(())
    }

    fn draw_anchor(
        &self,
        pixels: &mut [u8],
        x: u32,
        y: u32,
        size: u32,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let unit = size / 7;
        let pattern = [1, 1, 3, 1, 1]; // Ratio for scale invariance
        let mut offset = 0;

        for (i, &count) in pattern.iter().enumerate() {
            let color = if i % 2 == 0 { 0 } else { 255 }; // Alternate black/white

            for _ in 0..count {
                for py in 0..unit {
                    for px in 0..unit {
                        let pixel_x = x + offset + px;
                        let pixel_y = y + py;
                        if pixel_x < self.config.frame_width && pixel_y < self.config.frame_height {
                            let idx = ((pixel_y * self.config.frame_width + pixel_x) * 3) as usize;
                            pixels[idx] = color; // R
                            pixels[idx + 1] = color; // G
                            pixels[idx + 2] = color; // B
                        }
                    }
                }
                offset += unit;
            }
        }

        Ok(())
    }

    fn draw_header(
        &self,
        _pixels: &mut [u8],
        frame: &Frame,
    ) -> Result<(), Box<dyn std::error::Error>> {
        // Reserve top row of blocks for frame header
        // Frame ID + block size info
        println!(
            "[Frame {}] Header: ID={}, BlockSize={}",
            frame.header.frame_id, frame.header.frame_id, frame.header.block_size
        );
        Ok(())
    }

    fn draw_metadata(
        &self,
        _pixels: &mut [u8],
        metadata: &MetadataBlock,
    ) -> Result<(), Box<dyn std::error::Error>> {
        println!(
            "[Frame] Metadata: file={}, hash={:x?}",
            metadata.filename, metadata.file_hash
        );
        Ok(())
    }

    fn encode_payload(
        &self,
        pixels: &mut [u8],
        data: &[u8],
        _interleaving_map: &InterleavingMap,
    ) -> Result<(), Box<dyn std::error::Error>> {
        // For now, store data directly (will use interleaving map for actual implementation)
        // This is a simplified version - real implementation would use interleaving

        let mut data_idx = 0;
        let mut block_idx = 0;

        // Skip anchor regions (corners)
        const ANCHOR_MARGIN: u32 = 32;
        const HEADER_HEIGHT: u32 = 32;

        for block_y in (HEADER_HEIGHT / self.config.block_size as u32)
            ..((self.config.frame_height - ANCHOR_MARGIN) / self.config.block_size as u32)
        {
            for block_x in (ANCHOR_MARGIN / self.config.block_size as u32)
                ..((self.config.frame_width - ANCHOR_MARGIN) / self.config.block_size as u32)
            {
                if data_idx >= data.len() {
                    break;
                }

                let byte_val = data[data_idx];

                // Encode byte as 3 color values (3 bits per block = 8 colors)
                let color1 = (byte_val & 0b11100000) >> 5; // Bits 7-5
                let color2 = (byte_val & 0b00011100) >> 2; // Bits 4-2
                let color3 = (byte_val & 0b00000011) << 1; // Bits 1-0

                // Write the three colors to three sub-blocks
                let px = block_x * self.config.block_size as u32;
                let py = block_y * self.config.block_size as u32;

                self.set_block_color(pixels, px, py, color1)?;
                self.set_block_color(pixels, px + self.config.block_size as u32 / 2, py, color2)?;
                self.set_block_color(pixels, px, py + self.config.block_size as u32 / 2, color3)?;

                data_idx += 1;
                block_idx += 1;
            }
        }

        println!(
            "[Encoder] Encoded {} bytes into {} blocks",
            data_idx, block_idx
        );
        Ok(())
    }

    fn set_block_color(
        &self,
        pixels: &mut [u8],
        x: u32,
        y: u32,
        color_idx: u8,
    ) -> Result<(), Box<dyn std::error::Error>> {
        // Map color index (0-7) to RGB
        let (r, g, b) = match color_idx {
            0 => (0, 0, 0),       // Black
            1 => (255, 0, 0),     // Red
            2 => (0, 255, 0),     // Green
            3 => (255, 255, 0),   // Yellow
            4 => (0, 0, 255),     // Blue
            5 => (255, 0, 255),   // Magenta
            6 => (0, 255, 255),   // Cyan
            7 => (255, 255, 255), // White
            _ => (128, 128, 128), // Gray (fallback)
        };

        // Fill block with the color
        let block_size = self.config.block_size as u32;
        for brick_y in 0..block_size {
            for brick_x in 0..block_size {
                let px = x + brick_x;
                let py = y + brick_y;
                if px < self.config.frame_width && py < self.config.frame_height {
                    let idx = ((py * self.config.frame_width + px) * 3) as usize;
                    pixels[idx] = r;
                    pixels[idx + 1] = g;
                    pixels[idx + 2] = b;
                }
            }
        }

        Ok(())
    }
}

/// Output of encoding
pub struct EncodedOutput {
    pub frames: Vec<Frame>,
    pub token: String,
    pub metadata: MetadataBlock,
    pub num_frames: u32,
}

impl EncodedOutput {
    pub fn save_as_images(&self, output_dir: &str) -> Result<(), Box<dyn std::error::Error>> {
        fs::create_dir_all(output_dir)?;

        for (idx, frame) in self.frames.iter().enumerate() {
            let filename = format!("{}/frame_{:04}.png", output_dir, idx);

            if let Some(pixels) = &frame.pixel_data {
                self.save_frame_png(&filename, pixels, frame.frame_width, frame.frame_height)?;
                println!("[Encoder] Saved frame {}: {}", idx, filename);
            }
        }

        // Save token and metadata
        let metadata_json = serde_json::json!({
            "token": self.token,
            "filename": self.metadata.filename,
            "file_size": self.metadata.file_size,
            "file_hash": format!("{:x?}", self.metadata.file_hash),
            "num_frames": self.num_frames,
        });

        let metadata_path = format!("{}/metadata.json", output_dir);
        fs::write(metadata_path, metadata_json.to_string())?;
        println!("[Encoder] Metadata saved to metadata.json");

        Ok(())
    }

    fn save_frame_png(
        &self,
        filename: &str,
        pixels: &[u8],
        width: u32,
        height: u32,
    ) -> Result<(), Box<dyn std::error::Error>> {
        use image::{ImageBuffer, Rgb};

        let mut img = ImageBuffer::new(width, height);

        for (i, chunk) in pixels.chunks(3).enumerate() {
            let x = (i as u32) % width;
            let y = (i as u32) / width;
            if y < height {
                let rgb = Rgb([chunk[0], chunk[1], chunk[2]]);
                img.put_pixel(x, y, rgb);
            }
        }

        img.save(filename)?;
        Ok(())
    }
}

/// Generate a unique token for this encoding
fn generate_token() -> String {
    let uuid = Uuid::new_v4();
    format!("vshield://{}", uuid)
}

/// Encrypt data using ChaCha20-Poly1305
fn encrypt_data(data: &[u8], token: &str) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    use chacha20poly1305::aead::KeyInit;

    // Derive a 32-byte key from the token
    let mut hasher = Sha256::new();
    hasher.update(token.as_bytes());
    let key_bytes: [u8; 32] = hasher.finalize().into();

    let cipher = ChaCha20Poly1305::new(key_bytes[..].into());

    // Use a fixed nonce (in production, this should be derived/stored)
    let nonce = Nonce::from([0u8; 12]);

    let ciphertext = cipher
        .encrypt(&nonce, data)
        .map_err(|e| format!("Encryption failed: {}", e))?;

    Ok(ciphertext)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_token_generation() {
        let token1 = generate_token();
        let token2 = generate_token();

        assert!(token1.starts_with("vshield://"));
        assert_ne!(token1, token2);
    }

    #[test]
    fn test_encryption() {
        let data = b"Hello, V-Shield!";
        let token = "test-token";

        let encrypted = encrypt_data(data, token).unwrap();
        assert_ne!(&encrypted[..], data);
        assert!(!encrypted.is_empty());
    }
}
