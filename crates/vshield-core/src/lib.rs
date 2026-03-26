//! V-Shield Core Library
//! 
//! A comprehensive toolkit for encoding and decoding data into/from
//! video frames resistant to YouTube compression artifacts.

pub mod protocol;
pub mod anchor;
pub mod interleave;
pub mod ecc;

pub use protocol::{Frame, FrameHeader, MetadataBlock, ColorValue};
pub use anchor::{DetectedAnchor, AnchorPosition};
pub use interleave::InterleavingStrategy;
pub use ecc::{ECCConfig, RSEncoder};

/// Library version
pub const VERSION: &str = "0.1.0";
pub const PROTOCOL_VERSION: u8 = 1;

#[cfg(test)]
mod integration_tests {
    use super::*;

    #[test]
    fn test_frame_creation() {
        let frame = Frame::new(1, 8, 1920, 1080, true);
        assert_eq!(frame.header.frame_id, 1);
        assert_eq!(frame.header.block_size, 8);
        assert!(frame.header.is_first_frame);
    }
}
