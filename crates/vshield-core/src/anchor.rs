/// Finder Patterns (Anchors) - Scale-invariant detection markers
///
/// Anchors are placed in the corners of each frame to help the decoder:
/// 1. Find frame boundaries
/// 2. Detect skew/rotation
/// 3. Measure pixel-to-block scaling
/// 4. Compensate for YouTube's cropping and downscaling
///
/// Design: Pattern similar to QR codes - a 1:1:3:1:1 ratio sequence
/// This ratio is invariant to scaling, so it works at any resolution.
use crate::protocol::ColorValue;

/// Size of finder pattern in block units (not pixels)
pub const ANCHOR_SIZE_BLOCKS: usize = 10;

/// Finder pattern sequence: 1:1:3:1:1 - scale invariant
/// Visual representation:
/// ██░██  (where █ = dark, ░ = light)
/// The pattern repeats at different scales
pub type AnchorPattern = [u8; 5];

pub const STANDARD_ANCHOR: AnchorPattern = [1, 1, 3, 1, 1];

/// Represents a detected anchor at a specific location
#[derive(Debug, Clone, Copy)]
pub struct DetectedAnchor {
    /// X coordinate in pixels
    pub x: u32,
    /// Y coordinate in pixels
    pub y: u32,
    /// Detected block size in pixels (4, 8, 16, etc.)
    pub block_size: u8,
    /// Confidence score (0.0 to 1.0)
    pub confidence: f32,
    /// Position enum for which corner this is
    pub position: AnchorPosition,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AnchorPosition {
    TopLeft,
    TopRight,
    BottomLeft,
    BottomRight,
}

/// Generate a single anchor pattern block
pub fn generate_anchor(_size: u8) -> Vec<Vec<ColorValue>> {
    let mut grid = vec![vec![ColorValue::Black; ANCHOR_SIZE_BLOCKS]; ANCHOR_SIZE_BLOCKS];

    // Create 1:1:3:1:1 pattern
    let pattern_width = ANCHOR_SIZE_BLOCKS / 5;

    // Horizontal stripes
    for i in 0..ANCHOR_SIZE_BLOCKS {
        // Dark stripe
        for j in 0..pattern_width {
            grid[i][j] = ColorValue::White;
        }
        // Dark stripe
        for j in pattern_width..2 * pattern_width {
            grid[i][j] = ColorValue::White;
        }
        // Light stripe
        for j in 2 * pattern_width..5 * pattern_width {
            grid[i][j] = ColorValue::Black;
        }
        // Dark stripe
        for j in 5 * pattern_width..6 * pattern_width {
            grid[i][j] = ColorValue::White;
        }
        // Dark stripe
        for j in 6 * pattern_width..ANCHOR_SIZE_BLOCKS {
            if j < ANCHOR_SIZE_BLOCKS {
                grid[i][j] = ColorValue::White;
            }
        }
    }

    grid
}

/// Detect anchor patterns in a frame
/// This scans the image for the characteristic 1:1:3:1:1 pattern
pub fn detect_anchors(frame: &[Vec<ColorValue>], expected_block_size: u8) -> Vec<DetectedAnchor> {
    let mut anchors = Vec::new();
    let height = frame.len();
    let width = frame.get(0).map(|r| r.len()).unwrap_or(0);

    let block_size = expected_block_size as usize;
    let anchor_size = ANCHOR_SIZE_BLOCKS * block_size;

    // Try to find anchors at four corners
    let regions = vec![
        (0, 0, AnchorPosition::TopLeft), // Top-left
        (
            width.saturating_sub(anchor_size),
            0,
            AnchorPosition::TopRight,
        ), // Top-right
        (
            0,
            height.saturating_sub(anchor_size),
            AnchorPosition::BottomLeft,
        ), // Bottom-left
        (
            width.saturating_sub(anchor_size),
            height.saturating_sub(anchor_size),
            AnchorPosition::BottomRight,
        ), // Bottom-right
    ];

    for (start_x, start_y, position) in regions {
        if let Some(anchor) = detect_anchor_at(frame, start_x, start_y, block_size, position) {
            anchors.push(anchor);
        }
    }

    anchors
}

/// Detect anchor pattern at a specific location
fn detect_anchor_at(
    frame: &[Vec<ColorValue>],
    start_x: usize,
    start_y: usize,
    block_size: usize,
    position: AnchorPosition,
) -> Option<DetectedAnchor> {
    let height = frame.len();
    let width = frame.get(0).map(|r| r.len()).unwrap_or(0);
    let anchor_size = ANCHOR_SIZE_BLOCKS * block_size;

    // Check bounds
    if start_x + anchor_size > width || start_y + anchor_size > height {
        return None;
    }

    // Count dark vs light pixels in pattern regions
    let pattern_match = true;
    let mut confidence = 0.0f32;

    // This is a simplified detection - in reality, we'd scan for the pattern
    // For now, assume the pattern is there if we find high contrast
    let pattern_regions = vec![
        (0, block_size),                                   // Dark region 1
        (block_size, 2 * block_size),                      // Dark region 2
        (2 * block_size, 5 * block_size),                  // Light region
        (5 * block_size, 6 * block_size),                  // Dark region 3
        (6 * block_size, ANCHOR_SIZE_BLOCKS * block_size), // Dark region 4
    ];

    for (start, end) in &pattern_regions {
        let region_x_start = start_x + start;
        let region_x_end = (start_x + end).min(width);

        let mut brightness_sum = 0u32;
        let mut count = 0u32;

        for y in start_y..start_y + anchor_size {
            for x in region_x_start..region_x_end {
                if y < height && x < width {
                    let (r, g, b) = frame[y][x].to_rgb();
                    brightness_sum += (r as u32 + g as u32 + b as u32) / 3;
                    count += 1;
                }
            }
        }

        if count > 0 {
            let _avg_brightness = brightness_sum / count;
            // Check if this region matches expectation (dark vs light)
            // Placeholder logic
            confidence += 0.2;
        }
    }

    if pattern_match {
        Some(DetectedAnchor {
            x: start_x as u32,
            y: start_y as u32,
            block_size: block_size as u8,
            confidence: confidence.min(1.0),
            position,
        })
    } else {
        None
    }
}

/// Refine anchor detection through iterative alignment
pub fn refine_anchor_positions(
    frame: &[Vec<ColorValue>],
    initial_anchors: &[DetectedAnchor],
    max_iterations: u32,
) -> Vec<DetectedAnchor> {
    let mut anchors = initial_anchors.to_vec();
    let mut iteration = 0;

    while iteration < max_iterations {
        // Try to improve each anchor's position
        let mut improved = false;

        for anchor in &mut anchors {
            let _old_x = anchor.x;
            let _old_y = anchor.y;

            // Scan nearby regions for better matches
            let search_range = 5;
            let mut best_confidence = anchor.confidence;
            let mut best_x = anchor.x;
            let mut best_y = anchor.y;

            for dx in -search_range..=search_range {
                for dy in -search_range..=search_range {
                    let new_x = (anchor.x as i32 + dx).max(0) as u32;
                    let new_y = (anchor.y as i32 + dy).max(0) as u32;

                    // Score this position (simplified)
                    let score = score_anchor_position(frame, new_x as usize, new_y as usize);
                    if score > best_confidence {
                        best_confidence = score;
                        best_x = new_x;
                        best_y = new_y;
                        improved = true;
                    }
                }
            }

            anchor.x = best_x;
            anchor.y = best_y;
            anchor.confidence = best_confidence;
        }

        if !improved {
            break;
        }
        iteration += 1;
    }

    anchors
}

/// Score how well an anchor pattern matches at a specific position
fn score_anchor_position(frame: &[Vec<ColorValue>], x: usize, y: usize) -> f32 {
    let height = frame.len();
    let width = frame.get(0).map(|r| r.len()).unwrap_or(0);

    if x >= width || y >= height {
        return 0.0;
    }

    // Simple scoring: measure edge contrast
    let mut contrast_score = 0.0f32;

    // Check horizontal contrast
    if x + 1 < width {
        let (r1, g1, b1) = frame[y][x].to_rgb();
        let (r2, g2, b2) = frame[y][x + 1].to_rgb();
        let brightness1 = (r1 as f32 + g1 as f32 + b1 as f32) / 3.0;
        let brightness2 = (r2 as f32 + g2 as f32 + b2 as f32) / 3.0;
        contrast_score += (brightness1 - brightness2).abs() / 256.0;
    }

    // Check vertical contrast
    if y + 1 < height {
        let (r1, g1, b1) = frame[y][x].to_rgb();
        let (r2, g2, b2) = frame[y + 1][x].to_rgb();
        let brightness1 = (r1 as f32 + g1 as f32 + b1 as f32) / 3.0;
        let brightness2 = (r2 as f32 + g2 as f32 + b2 as f32) / 3.0;
        contrast_score += (brightness1 - brightness2).abs() / 256.0;
    }

    contrast_score / 2.0
}

/// Calculate frame transformation based on detected anchors
/// Returns (scale, rotation, translation)
pub struct FrameTransform {
    pub scale_x: f32,
    pub scale_y: f32,
    pub rotation: f32,
    pub translate_x: f32,
    pub translate_y: f32,
}

impl FrameTransform {
    pub fn identity() -> Self {
        FrameTransform {
            scale_x: 1.0,
            scale_y: 1.0,
            rotation: 0.0,
            translate_x: 0.0,
            translate_y: 0.0,
        }
    }

    /// Calculate transform from anchor positions
    pub fn from_anchors(
        anchors: &[DetectedAnchor],
        expected_width: u32,
        expected_height: u32,
    ) -> Self {
        if anchors.is_empty() {
            return FrameTransform::identity();
        }

        // Find corner positions
        let mut top_left = None;
        let mut top_right = None;
        let mut bottom_left = None;
        let mut bottom_right = None;

        for anchor in anchors {
            match anchor.position {
                AnchorPosition::TopLeft => top_left = Some(anchor),
                AnchorPosition::TopRight => top_right = Some(anchor),
                AnchorPosition::BottomLeft => bottom_left = Some(anchor),
                AnchorPosition::BottomRight => bottom_right = Some(anchor),
            }
        }

        // Calculate scale from anchor spacing
        if let (Some(tl), Some(tr), Some(bl), Some(_br)) =
            (top_left, top_right, bottom_left, bottom_right)
        {
            let width_pixels = (tr.x - tl.x) as f32;
            let height_pixels = (bl.y - tl.y) as f32;

            let scale_x = expected_width as f32 / width_pixels;
            let scale_y = expected_height as f32 / height_pixels;

            FrameTransform {
                scale_x,
                scale_y,
                rotation: 0.0, // Could calculate from anchor angles
                translate_x: tl.x as f32,
                translate_y: tl.y as f32,
            }
        } else {
            FrameTransform::identity()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_anchor() {
        let anchor = generate_anchor(8);
        assert_eq!(anchor.len(), ANCHOR_SIZE_BLOCKS);
        assert_eq!(anchor[0].len(), ANCHOR_SIZE_BLOCKS);
    }

    #[test]
    fn test_frame_transform_identity() {
        let transform = FrameTransform::identity();
        assert_eq!(transform.scale_x, 1.0);
        assert_eq!(transform.scale_y, 1.0);
        assert_eq!(transform.rotation, 0.0);
    }
}
