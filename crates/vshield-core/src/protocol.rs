/// V-Shield Frame Protocol
///
/// This module defines the complete protocol for encoding data into video frames.
///
/// Frame Structure:
/// ├── Finder Patterns (Anchors) - 4 corners
/// ├── Frame Header - metadata about the frame
/// ├── Metadata Block - (first frame only) file info, hash, token
/// └── Payload - actual data blocks with interleaving and ECC
use serde::{Deserialize, Serialize};
use std::fmt;

/// Default frame dimensions for encoding
pub const DEFAULT_FRAME_WIDTH: u32 = 1920;
pub const DEFAULT_FRAME_HEIGHT: u32 = 1080;

/// Default block size in pixels (can be 4x4, 8x8, or 16x16)
pub const DEFAULT_BLOCK_SIZE: u8 = 8;

/// Number of color values per pixel in our encoding
/// Using 4-8 colors to represent 2-3 bits per block
pub const COLOR_PALETTE_SIZE: usize = 8;

/// Finder pattern size (in blocks, not pixels)
pub const ANCHOR_BLOCK_SIZE: u8 = 10;

/// Error correction redundancy percentage
pub const ECC_REDUNDANCY_PERCENT: u8 = 25;

/// Maximum iterations for anchor detection
pub const MAX_ANCHOR_REFINEMENT: u32 = 10;

/// Frame header constants
pub const FRAME_HEADER_HEIGHT_BLOCKS: usize = 1; // 1 row of blocks for header

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum ColorValue {
    Black = 0,
    DarkGray = 1,
    Gray = 2,
    LightGray = 3,
    White = 4,
    DarkRed = 5,
    DarkBlue = 6,
    DarkGreen = 7,
}

impl ColorValue {
    /// Convert to RGB values for rendering
    pub fn to_rgb(&self) -> (u8, u8, u8) {
        match self {
            ColorValue::Black => (0, 0, 0),
            ColorValue::DarkGray => (64, 64, 64),
            ColorValue::Gray => (128, 128, 128),
            ColorValue::LightGray => (192, 192, 192),
            ColorValue::White => (255, 255, 255),
            ColorValue::DarkRed => (128, 0, 0),
            ColorValue::DarkBlue => (0, 0, 128),
            ColorValue::DarkGreen => (0, 128, 0),
        }
    }

    /// Convert to YUV values (better for video compression resistance)
    /// Y = 0.299*R + 0.587*G + 0.114*B
    /// U = -0.14713*R - 0.28886*G + 0.436*B
    /// V = 0.615*R - 0.51498*G - 0.10001*B
    pub fn to_yuv(&self) -> (u8, u8, u8) {
        let (r, g, b) = self.to_rgb();
        let r = r as f32;
        let g = g as f32;
        let b = b as f32;

        let y = (0.299 * r + 0.587 * g + 0.114 * b) as u8;
        let u = ((128.0 - 0.14713 * r - 0.28886 * g + 0.436 * b) as i16)
            .max(0)
            .min(255) as u8;
        let v = ((128.0 + 0.615 * r - 0.51498 * g - 0.10001 * b) as i16)
            .max(0)
            .min(255) as u8;

        (y, u, v)
    }

    /// Convert from RGB
    pub fn from_rgb(r: u8, g: u8, b: u8) -> Self {
        // Simple nearest-neighbor matching
        let (_or, _og, _ob) = ColorValue::Gray.to_rgb();
        let mut min_dist = i32::MAX;
        let mut closest = ColorValue::Black;

        for &color in &[
            ColorValue::Black,
            ColorValue::DarkGray,
            ColorValue::Gray,
            ColorValue::LightGray,
            ColorValue::White,
            ColorValue::DarkRed,
            ColorValue::DarkBlue,
            ColorValue::DarkGreen,
        ] {
            let (cr, cg, cb) = color.to_rgb();
            let dist = (r as i32 - cr as i32).pow(2)
                + (g as i32 - cg as i32).pow(2)
                + (b as i32 - cb as i32).pow(2);
            if dist < min_dist {
                min_dist = dist;
                closest = color;
            }
        }
        closest
    }
}

impl fmt::Display for ColorValue {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            ColorValue::Black => write!(f, "Black"),
            ColorValue::DarkGray => write!(f, "DarkGray"),
            ColorValue::Gray => write!(f, "Gray"),
            ColorValue::LightGray => write!(f, "LightGray"),
            ColorValue::White => write!(f, "White"),
            ColorValue::DarkRed => write!(f, "DarkRed"),
            ColorValue::DarkBlue => write!(f, "DarkBlue"),
            ColorValue::DarkGreen => write!(f, "DarkGreen"),
        }
    }
}

/// Frame header containing metadata about the frame structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FrameHeader {
    /// Unique identifier for this frame (prevents duplication/loss)
    pub frame_id: u32,
    /// Size of data blocks in pixels (4, 8, or 16)
    pub block_size: u8,
    /// Total number of data blocks in the frame (excluding header and anchors)
    pub data_blocks_count: u16,
    /// Is this the first frame (contains metadata)?
    pub is_first_frame: bool,
    /// Version of the protocol
    pub protocol_version: u8,
    /// Reserved flags for future use
    pub flags: u8,
}

impl FrameHeader {
    pub fn new(
        frame_id: u32,
        block_size: u8,
        data_blocks_count: u16,
        is_first_frame: bool,
    ) -> Self {
        FrameHeader {
            frame_id,
            block_size,
            data_blocks_count,
            is_first_frame,
            protocol_version: 1,
            flags: 0,
        }
    }

    /// Encode header to bytes (fixed 16 bytes)
    pub fn to_bytes(&self) -> [u8; 16] {
        let mut bytes = [0u8; 16];
        bytes[0..4].copy_from_slice(&self.frame_id.to_le_bytes());
        bytes[4] = self.block_size;
        bytes[5..7].copy_from_slice(&self.data_blocks_count.to_le_bytes());
        bytes[7] = if self.is_first_frame { 1 } else { 0 };
        bytes[8] = self.protocol_version;
        bytes[9] = self.flags;
        // bytes[10..16] reserved
        bytes
    }

    /// Decode header from bytes
    pub fn from_bytes(bytes: &[u8; 16]) -> Result<Self, String> {
        if bytes.len() < 10 {
            return Err("Header bytes too short".to_string());
        }
        Ok(FrameHeader {
            frame_id: u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]),
            block_size: bytes[4],
            data_blocks_count: u16::from_le_bytes([bytes[5], bytes[6]]),
            is_first_frame: bytes[7] != 0,
            protocol_version: bytes[8],
            flags: bytes[9],
        })
    }
}

/// Metadata block stored in the first frame
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetadataBlock {
    /// Original filename
    pub filename: String,
    /// Original file size in bytes
    pub file_size: u64,
    /// SHA-256 hash of original file (32 bytes)
    pub file_hash: [u8; 32],
    /// Unique token/key identifier (UUID as string, Base58 encoded in practice)
    pub token_id: String,
}

impl MetadataBlock {
    pub fn to_bytes(&self) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
        let json = serde_json::to_string(self)?;
        Ok(json.into_bytes())
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Self, Box<dyn std::error::Error>> {
        let json = String::from_utf8(bytes.to_vec())?;
        Ok(serde_json::from_str(&json)?)
    }
}

/// Represents a single data block that contains encoded bits
#[derive(Debug, Clone, Copy)]
pub struct DataBlock {
    /// 2D grid of colors, block_size x block_size
    pub pixels: [[ColorValue; 16]; 16], // Max 16x16
    /// Actual block size (4, 8, or 16)
    pub size: u8,
}

impl DataBlock {
    pub fn new(size: u8) -> Self {
        DataBlock {
            pixels: [[ColorValue::Black; 16]; 16],
            size,
        }
    }

    /// Encode bits into this block
    /// For 8 color palette, we can store 3 bits per block (2^3 = 8 colors)
    pub fn encode(&mut self, value: u8) {
        // Simple encoding: use the color value directly
        if value < COLOR_PALETTE_SIZE as u8 {
            let color = match value {
                0 => ColorValue::Black,
                1 => ColorValue::DarkGray,
                2 => ColorValue::Gray,
                3 => ColorValue::LightGray,
                4 => ColorValue::White,
                5 => ColorValue::DarkRed,
                6 => ColorValue::DarkBlue,
                7 => ColorValue::DarkGreen,
                _ => ColorValue::Black,
            };
            // Fill entire block with the color
            for i in 0..self.size as usize {
                for j in 0..self.size as usize {
                    self.pixels[i][j] = color;
                }
            }
        }
    }

    /// Decode bits from this block
    pub fn decode(&self) -> u8 {
        // Simple decoding: find the most common color
        let mut color_counts = [0u32; 8];
        for i in 0..self.size as usize {
            for j in 0..self.size as usize {
                match self.pixels[i][j] {
                    ColorValue::Black => color_counts[0] += 1,
                    ColorValue::DarkGray => color_counts[1] += 1,
                    ColorValue::Gray => color_counts[2] += 1,
                    ColorValue::LightGray => color_counts[3] += 1,
                    ColorValue::White => color_counts[4] += 1,
                    ColorValue::DarkRed => color_counts[5] += 1,
                    ColorValue::DarkBlue => color_counts[6] += 1,
                    ColorValue::DarkGreen => color_counts[7] += 1,
                }
            }
        }

        let mut max_count = 0;
        let mut max_color = 0;
        for (i, &count) in color_counts.iter().enumerate() {
            if count > max_count {
                max_count = count;
                max_color = i as u8;
            }
        }
        max_color
    }
}

/// Complete frame with all components
#[derive(Debug, Clone)]
pub struct Frame {
    pub header: FrameHeader,
    pub metadata: Option<MetadataBlock>,
    pub data_blocks: Vec<DataBlock>,
    /// Width and height in pixels
    pub frame_width: u32,
    pub frame_height: u32,
    /// Raw pixel data (RGB: 3 bytes per pixel)
    pub pixel_data: Option<Vec<u8>>,
}

impl Frame {
    pub fn new(
        frame_id: u32,
        block_size: u8,
        width: u32,
        height: u32,
        is_first_frame: bool,
    ) -> Self {
        let blocks_per_row = (width / block_size as u32) as u16;
        let blocks_per_col = (height / block_size as u32) as u16;
        let data_blocks_count = blocks_per_row * blocks_per_col - FRAME_HEADER_HEIGHT_BLOCKS as u16;

        Frame {
            header: FrameHeader::new(frame_id, block_size, data_blocks_count, is_first_frame),
            metadata: None,
            data_blocks: Vec::new(),
            frame_width: width,
            frame_height: height,
            pixel_data: None,
        }
    }

    /// Calculate how many bytes this frame can hold
    pub fn capacity(&self) -> usize {
        // Each color value = 3 bits (log2(8))
        // Each block can hold multiple pixels of data
        let bytes_per_block = 1; // Simplified: 1 byte per block for now
        self.header.data_blocks_count as usize * bytes_per_block
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_color_rgb_conversion() {
        let (r, g, b) = ColorValue::White.to_rgb();
        assert_eq!((r, g, b), (255, 255, 255));
    }

    #[test]
    fn test_color_yuv_conversion() {
        let (y, u, v) = ColorValue::White.to_yuv();
        assert!(y >= 240); // White should be bright
    }

    #[test]
    fn test_frame_header_serialization() {
        let header = FrameHeader::new(42, 8, 2048, true);
        let bytes = header.to_bytes();
        let decoded = FrameHeader::from_bytes(&bytes).unwrap();

        assert_eq!(decoded.frame_id, 42);
        assert_eq!(decoded.block_size, 8);
        assert_eq!(decoded.data_blocks_count, 2048);
        assert!(decoded.is_first_frame);
    }

    #[test]
    fn test_metadata_serialization() {
        let metadata = MetadataBlock {
            filename: "test.mp4".to_string(),
            file_size: 1024000,
            file_hash: [0u8; 32],
            token_id: "token-123".to_string(),
        };

        let bytes = metadata.to_bytes().unwrap();
        let decoded = MetadataBlock::from_bytes(&bytes).unwrap();

        assert_eq!(decoded.filename, "test.mp4");
        assert_eq!(decoded.file_size, 1024000);
    }
}
