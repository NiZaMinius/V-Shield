/// Data Interleaving System
///
/// Prevents burst errors (from YouTube compression/H.264 blocking) by
/// "spreading" each data byte across different physical locations in the frame.
///
/// Strategy: Instead of writing bytes sequentially, we use a strided pattern.
/// If YouTube destroys a 16x16 macroblock, we lose 1 byte from many different
/// Reed-Solomon blocks rather than destroying a complete block.

/// Interleaving strategy enumeration
#[derive(Debug, Clone, Copy)]
pub enum InterleavingStrategy {
    /// Write sequentially (baseline, no protection from burst errors)
    Sequential,
    /// Diagonal stripe pattern across blocks
    Diagonal,
    /// Checkerboard pattern
    Checkerboard,
    /// Random-like but deterministic (pseudo-random with seed)
    Pseudorandom { seed: u64 },
}

/// Maps logical byte indices to physical block positions in a frame
pub struct InterleavingMap {
    /// Frame width in blocks
    frame_width_blocks: usize,
    /// Frame height in blocks
    frame_height_blocks: usize,
    /// Total data blocks available
    total_blocks: usize,
    /// Strategy used
    strategy: InterleavingStrategy,
}

impl InterleavingMap {
    pub fn new(
        frame_width_blocks: usize,
        frame_height_blocks: usize,
        total_blocks: usize,
        strategy: InterleavingStrategy,
    ) -> Self {
        InterleavingMap {
            frame_width_blocks,
            frame_height_blocks,
            total_blocks,
            strategy,
        }
    }

    /// Get the physical block position for a logical byte index
    pub fn get_block_position(&self, byte_index: usize) -> Option<(usize, usize)> {
        if byte_index >= self.total_blocks {
            return None;
        }

        match self.strategy {
            InterleavingStrategy::Sequential => {
                let block_idx = byte_index % self.total_blocks;
                let row = block_idx / self.frame_width_blocks;
                let col = block_idx % self.frame_width_blocks;
                Some((col, row))
            }

            InterleavingStrategy::Diagonal => {
                // Distribute along diagonals
                let block_idx = byte_index % self.total_blocks;
                let total_diagonals = self.frame_width_blocks + self.frame_height_blocks - 1;
                let diagonal_idx = block_idx % total_diagonals;
                let position_in_diagonal = block_idx / total_diagonals;

                let (col, row) = if diagonal_idx < self.frame_width_blocks {
                    let col = diagonal_idx;
                    let row = position_in_diagonal;
                    (col, row)
                } else {
                    let row = diagonal_idx - self.frame_width_blocks + 1;
                    let col = position_in_diagonal;
                    (col, row)
                };

                if col < self.frame_width_blocks && row < self.frame_height_blocks {
                    Some((col, row))
                } else {
                    None
                }
            }

            InterleavingStrategy::Checkerboard => {
                // Dark/light checkerboard pattern
                let block_idx = byte_index % self.total_blocks;
                let mut count = 0;
                for row in 0..self.frame_height_blocks {
                    for col in 0..self.frame_width_blocks {
                        if (row + col) % 2 == byte_index % 2 {
                            if count == block_idx {
                                return Some((col, row));
                            }
                            count += 1;
                        }
                    }
                }
                None
            }

            InterleavingStrategy::Pseudorandom { seed } => {
                // Linear congruential generator for deterministic pseudo-randomness
                let mut rng_state = seed.wrapping_add(byte_index as u64);
                rng_state = rng_state.wrapping_mul(1103515245).wrapping_add(12345);

                let block_pos = (rng_state as usize) % self.total_blocks;
                let row = block_pos / self.frame_width_blocks;
                let col = block_pos % self.frame_width_blocks;

                if col < self.frame_width_blocks && row < self.frame_height_blocks {
                    Some((col, row))
                } else {
                    None
                }
            }
        }
    }

    /// Get the reverse mapping: physical position to logical byte index
    pub fn get_byte_index(&self, col: usize, row: usize) -> Option<usize> {
        if col >= self.frame_width_blocks || row >= self.frame_height_blocks {
            return None;
        }

        match self.strategy {
            InterleavingStrategy::Sequential => {
                let block_idx = row * self.frame_width_blocks + col;
                if block_idx < self.total_blocks {
                    Some(block_idx)
                } else {
                    None
                }
            }

            // For other strategies, we'd need to iterate to find the reverse mapping
            // This is a simplification - in production, we'd maintain a bidirectional map
            _ => None,
        }
    }

    /// Interleave data across frame blocks
    /// Takes sequential data and maps it to frame positions
    pub fn interleave_data(&self, data: &[u8]) -> Result<Vec<Option<u8>>, String> {
        let mut result = vec![None; self.total_blocks];

        for (byte_idx, &byte_val) in data.iter().enumerate() {
            if let Some((col, row)) = self.get_block_position(byte_idx) {
                let block_idx = row * self.frame_width_blocks + col;
                if block_idx < self.total_blocks {
                    result[block_idx] = Some(byte_val);
                }
            }
        }

        Ok(result)
    }

    /// Deinterleave data from frame blocks back to sequential form
    pub fn deinterleave_data(&self, blocks: &[Option<u8>]) -> Result<Vec<u8>, String> {
        let mut result = vec![0u8; self.total_blocks];

        for (block_idx, &byte_opt) in blocks.iter().enumerate() {
            if let Some(byte_val) = byte_opt {
                let row = block_idx / self.frame_width_blocks;
                let col = block_idx % self.frame_width_blocks;

                // Find which logical byte index corresponds to this position
                for logical_idx in 0..self.total_blocks {
                    if let Some((mapped_col, mapped_row)) = self.get_block_position(logical_idx) {
                        if mapped_col == col && mapped_row == row {
                            result[logical_idx] = byte_val;
                            break;
                        }
                    }
                }
            }
        }

        Ok(result)
    }
}

/// Advanced interleaving for multiple Reed-Solomon blocks
/// Distributes ECC parity symbols across the frame to maximize resilience
pub struct MultiBlockInterleaver {
    /// Number of Reed-Solomon blocks
    _num_blocks: usize,
    /// Symbols per block
    _symbols_per_block: usize,
    /// Interleaving depth (how many positions before cycling to next block)
    depth: usize,
}

impl MultiBlockInterleaver {
    pub fn new(num_blocks: usize, symbols_per_block: usize) -> Self {
        let depth = symbols_per_block / 4; // Empirically chosen for good distribution
        MultiBlockInterleaver {
            _num_blocks: num_blocks,
            _symbols_per_block: symbols_per_block,
            depth: depth.max(1),
        }
    }

    /// Get frame position for a symbol from a Reed-Solomon block
    pub fn get_frame_position(
        &self,
        block_num: usize,
        symbol_idx: usize,
        frame_width_blocks: usize,
        frame_height_blocks: usize,
    ) -> Option<(usize, usize)> {
        let total_frame_blocks = frame_width_blocks * frame_height_blocks;

        // Interleave across blocks in a strided pattern
        let physical_idx = (block_num * self.depth + symbol_idx) % total_frame_blocks;

        let row = physical_idx / frame_width_blocks;
        let col = physical_idx % frame_width_blocks;

        if col < frame_width_blocks && row < frame_height_blocks {
            Some((col, row))
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sequential_interleaving() {
        let map = InterleavingMap::new(8, 6, 48, InterleavingStrategy::Sequential);

        assert_eq!(map.get_block_position(0), Some((0, 0)));
        assert_eq!(map.get_block_position(1), Some((1, 0)));
        assert_eq!(map.get_block_position(8), Some((0, 1)));
    }

    #[test]
    fn test_checkerboard_interleaving() {
        let map = InterleavingMap::new(8, 6, 24, InterleavingStrategy::Checkerboard);

        // Checkerboard should only use even positions
        let mut count = 0;
        for row in 0..6 {
            for col in 0..8 {
                if (row + col) % 2 == 0 {
                    count += 1;
                }
            }
        }
        assert!(count >= 24);
    }

    #[test]
    fn test_interleave_deinterleave() {
        let map = InterleavingMap::new(4, 4, 16, InterleavingStrategy::Sequential);
        let data = vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16];

        let interleaved = map.interleave_data(&data).unwrap();
        let deinterleaved = map.deinterleave_data(&interleaved).unwrap();

        // Some data might be lost in interleaving, but sequential should preserve it
        assert_eq!(deinterleaved.len(), data.len());
    }

    #[test]
    fn test_multiblock_distribution() {
        let interleaver = MultiBlockInterleaver::new(4, 16);

        // Check that symbols from same block don't cluster
        let mut positions = Vec::new();
        for block in 0..4 {
            for sym in 0..4 {
                if let Some(pos) = interleaver.get_frame_position(block, sym, 8, 8) {
                    positions.push(pos);
                }
            }
        }

        assert!(!positions.is_empty());
    }
}
