/// Error Correction Code (ECC) Module
///
/// Implements Reed-Solomon error correction for resilience against
/// YouTube's compression artifacts and data loss.
///
/// Strategy: Add 20-30% redundancy. If up to K erasures occur,
/// RS can reconstruct all original data as long as K < redundancy_symbols.
use std::cmp::min;

use reed_solomon_erasure::galois_8::ReedSolomon;

/// Reed-Solomon configuration
#[derive(Debug, Clone, Copy)]
pub struct ECCConfig {
    /// Total number of symbols (data + parity)
    pub total_symbols: usize,
    /// Number of data symbols
    pub data_symbols: usize,
    /// Number of parity/redundancy symbols (= total - data)
    parity_symbols: usize,
}

impl ECCConfig {
    /// Create a new ECC configuration
    /// data_symbols: how many symbols of original data
    /// redundancy_percent: what % redundancy to add (typically 20-30)
    pub fn new(data_symbols: usize, redundancy_percent: u8) -> Self {
        let redundancy_percent = min(redundancy_percent, 100);
        let parity_symbols = (data_symbols * redundancy_percent as usize) / 100;

        ECCConfig {
            total_symbols: data_symbols + parity_symbols,
            data_symbols,
            parity_symbols,
        }
    }

    pub fn parity_symbols(&self) -> usize {
        self.parity_symbols
    }

    pub fn redundancy_percent(&self) -> u8 {
        if self.data_symbols == 0 {
            0
        } else {
            ((self.parity_symbols * 100) / self.data_symbols) as u8
        }
    }
}

/// Reed-Solomon Error Correction Codec
/// Uses the reed-solomon-erasure crate for actual encoding/decoding
pub struct RSEncoder {
    config: ECCConfig,
}

impl RSEncoder {
    pub fn new(config: ECCConfig) -> Result<Self, String> {
        // Validate RS parameters
        // Standard RS codes are typically (255, K) where K <= 255
        if config.total_symbols > 255 {
            return Err(format!(
                "RS code requires total_symbols <= 255, got {}",
                config.total_symbols
            ));
        }

        Ok(RSEncoder { config })
    }

    /// Get the configuration
    pub fn config(&self) -> ECCConfig {
        self.config
    }

    /// Encode data with Reed-Solomon parity symbols
    /// Takes `data_symbols` bytes and returns `total_symbols` bytes
    pub fn encode(&self, data: &[u8]) -> Result<Vec<u8>, String> {
        if data.len() != self.config.data_symbols {
            return Err(format!(
                "Expected {} data bytes, got {}",
                self.config.data_symbols,
                data.len()
            ));
        }

        let rs = ReedSolomon::new(self.config.data_symbols, self.config.parity_symbols)
            .map_err(|e| format!("Failed to create Reed-Solomon encoder: {:?}", e))?;

        let mut buffer = vec![0u8; self.config.total_symbols];

        // Set data shards
        buffer[..self.config.data_symbols].copy_from_slice(data);
        let mut shards: Vec<&mut [u8]> = buffer.chunks_mut(1).collect();

        // Compute parity
        rs.encode(&mut shards)
            .map_err(|e| format!("Reed-Solomon encoding failed: {:?}", e))?;

        Ok(buffer)
    }

    /// Decode data with potential erasures
    /// Pass `None` for erased symbols, `Some(byte)` for known symbols
    /// Returns reconstructed data if possible
    pub fn decode(&self, symbols: &[Option<u8>]) -> Result<Vec<u8>, String> {
        if symbols.len() != self.config.total_symbols {
            return Err(format!(
                "Expected {} symbols, got {}",
                self.config.total_symbols,
                symbols.len()
            ));
        }

        use reed_solomon_erasure::galois_8::ReedSolomon;

        let rs = ReedSolomon::new(self.config.data_symbols, self.config.parity_symbols)
            .map_err(|e| format!("Failed to create Reed-Solomon decoder: {:?}", e))?;

        let mut shards: Vec<Option<Vec<u8>>> =
            symbols.iter().map(|opt| opt.map(|b| vec![b])).collect();

        // Recover missing shards
        rs.reconstruct(&mut shards)
            .map_err(|e| format!("Reed-Solomon reconstruction failed: {:?}", e))?;

        // Extract data shards
        let mut result = Vec::new();
        for i in 0..self.config.data_symbols {
            if let Some(shard) = &shards[i] {
                if !shard.is_empty() {
                    result.push(shard[0]);
                } else {
                    return Err(format!("Data shard {} is empty", i));
                }
            } else {
                return Err(format!("Data shard {} could not be recovered", i));
            }
        }

        Ok(result)
    }

    /// Check if data can be recovered from the given erasure pattern
    pub fn can_recover(&self, erasure_count: usize) -> bool {
        erasure_count <= self.config.parity_symbols
    }

    /// Calculate the maximum number of erasures that can be corrected
    pub fn max_correctable_erasures(&self) -> usize {
        self.config.parity_symbols
    }
}

/// Multi-block decoder for handling multiple Reed-Solomon blocks
/// Important: Each block can be decoded independently, but
/// interleaving distributes them across the frame
pub struct MultiBlockDecoder {
    blocks: Vec<RSEncoder>,
}

impl MultiBlockDecoder {
    pub fn new(
        num_blocks: usize,
        data_symbols_per_block: usize,
        redundancy_percent: u8,
    ) -> Result<Self, String> {
        let mut blocks = Vec::new();
        for _ in 0..num_blocks {
            let config = ECCConfig::new(data_symbols_per_block, redundancy_percent);
            blocks.push(RSEncoder::new(config)?);
        }
        Ok(MultiBlockDecoder { blocks })
    }

    pub fn num_blocks(&self) -> usize {
        self.blocks.len()
    }

    /// Decode a single block
    pub fn decode_block(
        &self,
        block_idx: usize,
        symbols: &[Option<u8>],
    ) -> Result<Vec<u8>, String> {
        if block_idx >= self.blocks.len() {
            return Err(format!("Block index {} out of range", block_idx));
        }
        self.blocks[block_idx].decode(symbols)
    }

    /// Get total capacity with ECC
    pub fn total_capacity(&self) -> usize {
        self.blocks.iter().map(|b| b.config().total_symbols).sum()
    }

    /// Get raw data capacity (without ECC overhead)
    pub fn data_capacity(&self) -> usize {
        self.blocks.iter().map(|b| b.config().data_symbols).sum()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ecc_config_creation() {
        let config = ECCConfig::new(100, 25);
        assert_eq!(config.data_symbols, 100);
        assert_eq!(config.parity_symbols, 25);
        assert_eq!(config.total_symbols, 125);
    }

    #[test]
    fn test_ecc_config_redundancy() {
        let config = ECCConfig::new(200, 30);
        assert_eq!(config.redundancy_percent(), 30);
    }

    #[test]
    fn test_simple_encode_decode() -> Result<(), Box<dyn std::error::Error>> {
        let config = ECCConfig::new(10, 20);
        let encoder = RSEncoder::new(config)?;

        let data = vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10];
        let encoded = encoder.encode(&data)?;

        assert_eq!(encoded.len(), config.total_symbols);

        // Decode without erasures
        let symbols: Vec<Option<u8>> = encoded.iter().map(|&b| Some(b)).collect();
        let decoded = encoder.decode(&symbols)?;

        assert_eq!(decoded, data);
        Ok(())
    }

    #[test]
    fn test_encode_decode_with_erasures() -> Result<(), Box<dyn std::error::Error>> {
        let config = ECCConfig::new(10, 40); // 40% redundancy = 14 total, 4 parity
        let encoder = RSEncoder::new(config)?;

        let data = vec![10, 20, 30, 40, 50, 60, 70, 80, 90, 100];
        let encoded = encoder.encode(&data)?;

        // Simulate losing 2 bytes (within 4-byte parity capacity)
        let mut symbols: Vec<Option<u8>> = encoded.iter().map(|&b| Some(b)).collect();
        symbols[2] = None;
        symbols[5] = None;

        let decoded = encoder.decode(&symbols)?;
        assert_eq!(decoded, data);
        Ok(())
    }

    #[test]
    fn test_max_correctable_erasures() {
        let config = ECCConfig::new(100, 25);
        let encoder = RSEncoder::new(config).unwrap();
        assert_eq!(encoder.max_correctable_erasures(), 25);
    }

    #[test]
    fn test_multi_block_decoder() -> Result<(), Box<dyn std::error::Error>> {
        let decoder = MultiBlockDecoder::new(3, 10, 25)?;
        assert_eq!(decoder.num_blocks(), 3);
        assert_eq!(decoder.data_capacity(), 30);
        assert_eq!(decoder.total_capacity(), 36); // 3 blocks * (10 data + 2 parity)
        Ok(())
    }
}
