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
    /// Note: &[u8; 16] guarantees exactly 16 bytes at compile time
    pub fn from_bytes(bytes: &[u8; 16]) -> Result<Self, String> {
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
/// Fixed-size binary format (232 bytes): 128 (filename) + 8 (size) + 32 (hash) + 64 (token)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetadataBlock {
    /// Original filename (max 127 chars + null terminator)
    pub filename: String,
    /// Original file size in bytes
    pub file_size: u64,
    /// SHA-256 hash of original file (32 bytes)
    pub file_hash: [u8; 32],
    /// Unique token identifier (vshield://...)
    pub token_id: String,
}

impl MetadataBlock {
    /// Fixed size in bytes
    pub const SIZE: usize = 128 + 8 + 32 + 64;

    /// Serialize to fixed-size binary (232 bytes)
    pub fn to_bytes(&self) -> [u8; Self::SIZE] {
        let mut buf = [0u8; Self::SIZE];

        // filename: 128 bytes null-terminated
        let name = self.filename.as_bytes();
        let len = name.len().min(127);
        buf[..len].copy_from_slice(&name[..len]);

        // file_size: 8 bytes LE
        buf[128..136].copy_from_slice(&self.file_size.to_le_bytes());

        // file_hash: 32 bytes
        buf[136..168].copy_from_slice(&self.file_hash);

        // token_id: 64 bytes null-terminated
        let token = self.token_id.as_bytes();
        let tlen = token.len().min(63);
        buf[168..168 + tlen].copy_from_slice(&token[..tlen]);

        buf
    }

    /// Deserialize from fixed-size binary
    pub fn from_bytes(buf: &[u8]) -> Result<Self, String> {
        if buf.len() < Self::SIZE {
            return Err(format!(
                "Метаданные слишком короткие: {} < {}",
                buf.len(),
                Self::SIZE
            ));
        }

        let name_end = buf[..128].iter().position(|&b| b == 0).unwrap_or(128);
        let filename = String::from_utf8_lossy(&buf[..name_end]).to_string();

        let file_size = u64::from_le_bytes(
            buf[128..136]
                .try_into()
                .map_err(|_| "file_size error".to_string())?,
        );

        let mut file_hash = [0u8; 32];
        file_hash.copy_from_slice(&buf[136..168]);

        let token_end = buf[168..232].iter().position(|&b| b == 0).unwrap_or(64);
        let token_id = String::from_utf8_lossy(&buf[168..168 + token_end]).to_string();

        Ok(Self {
            filename,
            file_size,
            file_hash,
            token_id,
        })
    }
}

/// Single data block encodes 3 bits as one color
/// Entire block is uniform color (majority voted during decode)
#[derive(Debug, Clone, Copy)]
pub struct DataBlock {
    /// Color value (encodes 3 bits of data)
    pub color: ColorValue,
    /// Block size in pixels (4, 8, or 16)
    pub size: u8,
}

impl DataBlock {
    /// Create new block
    pub fn new(size: u8) -> Self {
        Self {
            color: ColorValue::Black,
            size,
        }
    }

    /// Encode 3 bits into this block
    pub fn encode(bits: u8, size: u8) -> Self {
        let color = match bits & 0b111 {
            0 => ColorValue::Black,
            1 => ColorValue::DarkGray,
            2 => ColorValue::Gray,
            3 => ColorValue::LightGray,
            4 => ColorValue::White,
            5 => ColorValue::DarkRed,
            6 => ColorValue::DarkBlue,
            7 => ColorValue::DarkGreen,
            _ => unreachable!(),
        };
        Self { color, size }
    }

    /// Decode using majority vote from actual pixel buffer
    pub fn decode_from_pixels(pixels: &[u8], stride: usize, x: u32, y: u32, size: u8) -> u8 {
        let mut counts = [0u32; 8];
        for dy in 0..size as usize {
            for dx in 0..size as usize {
                let idx = (y as usize + dy) * stride + (x as usize + dx) * 3;
                if idx + 2 < pixels.len() {
                    let color = ColorValue::from_rgb(pixels[idx], pixels[idx + 1], pixels[idx + 2]);
                    counts[color as usize] += 1;
                }
            }
        }
        counts
            .iter()
            .enumerate()
            .max_by_key(|(_, &c)| c)
            .map(|(i, _)| i as u8)
            .unwrap_or(0)
    }

    /// Get encoded bits from stored color
    pub fn decode(&self) -> u8 {
        self.color as u8
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

    /// Calculate data capacity in bytes
    /// Each block holds 3 bits (8-color palette), so divide total bits by 8
    pub fn capacity_bytes(&self) -> usize {
        (self.header.data_blocks_count as usize * 3) / 8
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
        assert!(u >= 120 && u <= 136, "U value out of expected range: {}", u);
        assert!(v >= 120 && v <= 136, "V value out of expected range: {}", v);
    }

    #[test]
    fn test_color_yuv_preservation() {
        // We check that the colors of the palette are distingushable by brightness (Y-channel)
        // We sort by Y and check that the adjacent ones are different enough
        let colors = vec![
            ColorValue::Black,
            ColorValue::DarkBlue,
            ColorValue::DarkRed,
            ColorValue::DarkGreen,
            ColorValue::DarkGray,
            ColorValue::Gray,
            ColorValue::LightGray,
            ColorValue::White,
        ];

        // Collecting Y-values
        let mut y_values: Vec<(ColorValue, u8)> =
            colors.iter().map(|&c| (c, c.to_yuv().0)).collect();

        // sort by Y
        y_values.sort_by_key(|(_, y)| *y);

        // Each adjacent color must differ by at least 10 in Y
        for pair in y_values.windows(2) {
            let (color_a, y_a) = pair[0];
            let (color_b, y_b) = pair[1];
            assert!(
                (y_b as i32 - y_a as i32) >= 10,
                "Colors {:?}(y={}) and {:?}(Y={}) too close in brightness",
                color_a,
                y_a,
                color_b,
                y_b
            );
        }
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

        let bytes = metadata.to_bytes();
        let decoded = MetadataBlock::from_bytes(&bytes).unwrap();

        assert_eq!(decoded.filename, "test.mp4");
        assert_eq!(decoded.file_size, 1024000);
    }
}
