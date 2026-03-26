# V-Shield Phase 1 Testing Guide

## Quick Start

### Prerequisites
```bash
# Build the project
cargo build --release
```

### Files Location
- **Test input files**: `test_files/`
  - `document.txt` - Plain text file (~500 bytes)
  - `config.json` - JSON configuration (~400 bytes)
  - `binary.bin` - Binary data (756 bytes)
  - `small.txt` - Small text file (~50 bytes)

- **Test output**: `output/` (created during encoding)
  - `test1_document/frame_*.png` - Encoded frames
  - `test2_config/frame_*.png` - Encoded frames
  - `test3_binary/frame_*.png` - Encoded frames
  - `decoded_*.txt|json|bin` - Recovered files

---

## Encoding Test Files

### Method 1: Windows Batch Script
```batch
encode_test.bat
```
This will:
1. Encode all test files
2. Display encoding results and tokens
3. **IMPORTANT**: Write down the tokens - you'll need them for decoding!

### Method 2: Manual Command
```bash
# Encode a text file
./target/release/vshield-encode.exe --input test_files/document.txt --output output/test1 --block-size 8 --redundancy 25
```

**When the encoder finishes, it displays:**
```
✅ Encoding complete!
   Frames:     1
   Token:      vshield://xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx
   File Hash:  [30, ca, 3b, f3, 26, e3, ...]
```

**SAVE THIS TOKEN** - you need it to decode the file!

### Encoder Output Structure
```
output/
├── test1_document/
│   ├── frame_0000.png    ← Your encoded frame (can view as image)
│   └── metadata.json     ← Frame information
├── test2_config/
│   ├── frame_0000.png
│   └── metadata.json
└── test3_binary/
    ├── frame_0000.png
    └── metadata.json
```

The PNG files show colorful 8×8 blocks which are the encoded data.

---

## Decoding Test Files (Recovery)

### Required Information
To decode, you need:
1. **Input directory**: Where the encoded frames are (e.g., `output/test1_document`)
2. **Token**: From the encoding step (e.g., `vshield://...`)
3. **Output path**: Where to save recovered file (e.g., `output/decoded_document.txt`)

### Method 1: Windows Interactive Batch
```batch
decode_test.bat
```

Prompts you to enter tokens and automatically decodes all files.

### Method 2: Manual Command
```bash
./target/release/vshield-decode.exe \
  --input output/test1_document \
  --output output/decoded_document.txt \
  --token "vshield://xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx"
```

Replace the token with your actual token from encoding!

### Recovery Output
```
output/
├── decoded_document.txt     ← Recovered (should be identical to original)
├── decoded_config.json      ← Recovered JSON
└── decoded_binary.bin       ← Recovered binary data
```

---

## Complete Test Workflow

### Full Automated Test (Linux/Bash)
```bash
./run_all_tests.sh
```

This script:
1. ✅ Encodes all test files
2. ✅ Automatically captures tokens
3. ✅ Decodes everything using saved tokens
4. ✅ Compares original vs recovered files
5. ✅ Reports PASS/FAIL for each test

### Step-by-Step Windows

**Step 1: Build**
```bash
cargo build --release
```

**Step 2: Encode**
```batch
encode_test.bat
```
- Follows the prompts
- **Write down the displayed tokens in a text file**

**Step 3: Decode**
```batch
decode_test.bat
```
- Paste each token when prompted
- Watch it recover the files

**Step 4: Verify**
```bash
# Compare files
fc test_files\document.txt output\decoded_document.txt

# Should show: "no differences encountered"
```

---

## Verification & Testing

### File Integrity Check
```bash
# Windows
fc test_files\document.txt output\decoded_document.txt

# Linux
cmp test_files/document.txt output/decoded_document.txt

# Expected: Files are identical
```

### Hash Verification
```bash
# Windows PowerShell
(Get-FileHash test_files\document.txt).Hash
(Get-FileHash output\decoded_document.txt).Hash

# Both hashes should be identical
```

### What You're Testing
- ✅ File encryption/decryption works
- ✅ Error correction protects data
- ✅ Frames encode correctly
- ✅ Data recovery is perfect (byte-for-byte)
- ✅ Token system prevents unauthorized access

---

## Encoder Options

### Default (Safe)
```bash
--block-size 8 --redundancy 25
```
- Good balance of speed and resilience
- Recommended for most files

### High Resilience (Slow)
```bash
--block-size 4 --redundancy 40
```
- Smaller blocks = more error correction
- Handles heavy YouTube compression
- Slower encoding

### High Speed (Less Resilient)
```bash
--block-size 16 --redundancy 20
```
- Larger blocks = faster encoding
- Less protection against errors
- Use for uncompressed media

---

## Understanding the Output

### What's in frame_0000.png?

The PNG file contains:
```
Top-left corner:  [Finder Pattern] - Used for frame detection
Top rows:         [Frame Header]   - Metadata about frame
Below header:     [Data Blocks]    - Your encrypted data as 8×8 color blocks
Bottom corner:    [Finder Pattern] - Detection redundancy
```

The colorful 8×8 blocks are your encoded data. Each color value (0-7) represents 3 bits.

### What's in metadata.json?
```json
{
  "filename": "document.txt",
  "file_size": 500,
  "file_hash": "30ca3bf3...",
  "num_frames": 1,
  "version": "0.1.0"
}
```

Used for verification and recovery.

### What's the Token?
```
vshield://efdc7b20-c0e6-47f5-85b4-64e706d8c92d
```
- **Unique for each file**
- Contains encryption key
- **Must be saved to recover file**
- Generated from file content + random UUID

---

## Test Cases

### Test 1: Plain Text (document.txt)
```
Input:    Plain English text (500 bytes)
Process:  UTF-8 → Encrypt → ECC → Frames
Output:   1 frame PNG + metadata
Recovery: Should be byte-identical
```

### Test 2: JSON (config.json)
```
Input:    Structured JSON data (400 bytes)
Process:  JSON text → Encrypt → ECC → Frames
Output:   1 frame PNG + metadata
Recovery: Should parse and match original
```

### Test 3: Binary Data (binary.bin)
```
Input:    All byte values 0-255 (756 bytes)
Process:  Binary → Encrypt → ECC → Frames
Output:   1 frame PNG + metadata
Recovery: Should be bit-identical
```

---

## Troubleshooting

### Problem: "Binaries not found"
```bash
# Solution
cargo build --release
```

### Problem: "Token not entered"
**Solution**: Run encoder again, save token this time

### Problem: "Decoded file doesn't match original"
1. Verify token is correct
2. Check input directory has all frames
3. Ensure no frame files are corrupted

### Problem: "Encoder is slow"
1. Use `--block-size 16` for faster encoding
2. Reduce redundancy to 20%

### Problem: "Cannot find test files"
```bash
# Create them
cd test_files
python3 ../create_test_files.py
```

---

## Results Summary

| Test | Input File | Size | Frames | Status | Verify |
|------|-----------|------|--------|--------|--------|
| 1 | document.txt | 500 B | 1 | Encode ✓ | `fc test_files\document.txt output\decoded_*.txt` |
| 2 | config.json | 400 B | 1 | Encode ✓ | `fc test_files\config.json output\decoded_*.json` |
| 3 | binary.bin | 756 B | 1 | Encode ✓ | `fc test_files\binary.bin output\decoded_*.bin` |

After completing all tests, each decoded file should match its original exactly.

---
    let decoded = FrameHeader::from_bytes(&bytes).unwrap();
    assert_eq!(decoded.frame_id, 42);
}
```

#### Anchor Pattern Tests
```rust
#[test]
fn test_anchor_generation() {
    // Verify anchors have correct 1:1:3:1:1 ratio
    let anchor = generate_anchor(8);
    assert_eq!(anchor.len(), 28); // 7*4 pixels
    // Verify pattern is black-white-black-white
}

#[test]
fn test_anchor_detection_at_scale() {
    // Test that anchors are detectable after downscaling
    let anchor = generate_anchor(8);
    // Scale to 50% (like 1080p → 540p)
    let scaled = downsample_2x(&anchor);
    // Verify still detectable
    assert!(find_anchor(&scaled).is_some());
}
```

#### Error Correction Tests
```rust
#[test]
fn test_reed_solomon_encoding_decoding() {
    let data = "Hello, V-Shield!".as_bytes();
    let encoder = RSEncoder::new(ECCConfig::new(data.len(), 25))?;
    let encoded = encoder.encode(data)?;
    
    // Should have 25% extra (16 bytes + 4)
    assert_eq!(encoded.len(), 20);
}

#[test]
fn test_error_correction_capacity() {
    // Reed-Solomon can correct up to (ecc_bytes / 2) symbol errors
    // With 25% redundancy on 256 symbols: can correct 32 symbols
    let mut corrupted = encoded.clone();
    corrupt_random_bytes(&mut corrupted, 30); // Corrupt 30 bytes
    
    let decoded = decoder.decode(&corrupted)?;
    assert_eq!(decoded, original_data);
}
```

### Encoder Tests

Located in: `crates/vshield-enc/src/`

```rust
#[test]
fn test_token_generation() {
    let token1 = generate_token();
    let token2 = generate_token();
    assert!(token1.starts_with("vshield://"));
    assert_ne!(token1, token2);
}

#[test]
fn test_encryption_roundtrip() {
    let plaintext = b"Test data";
    let token = "test-token";
    
    let ciphertext = encrypt_data(plaintext, token)?;
    assert_ne!(ciphertext, plaintext); // Should be different
    
    // Must be able to decrypt with same token
    let decrypted = decrypt_data(&ciphertext, token)?;
    assert_eq!(decrypted, plaintext);
}

#[test]
fn test_frame_capacity_calculation() {
    // 1920x1080, 8px blocks, 8 colors (3 bits)
    // = 240*135 blocks = 32,400 blocks
    // = 32,400 / 8 = 4,050 bytes per frame
    let encoder = Encoder::new(EncoderConfig::default());
    assert_eq!(encoder.frame_capacity(), 4050);
}
```

### Decoder Tests

```rust
#[test]
fn test_color_extraction() {
    // Create test block with known color
    let block = create_test_block(ColorValue::White);
    let decoded = block.decode();
    assert_eq!(decoded, 4); // White = value 4
}

#[test]
fn test_pixel_data_loading() {
    // Create test PNG, load it as Frame
    let img = create_test_image(1920, 1080);
    img.save("test_frame.png")?;
    
    let frame = load_frame_from_png("test_frame.png")?;
    assert_eq!(frame.frame_width, 1920);
    assert_eq!(frame.frame_height, 1080);
}
```

---

## Integration Tests

Run with:
```bash
cargo test --test '*'
```

Create file: `tests/encode_decode_cycle.rs`

### Complete Encode→Decode Roundtrip

```rust
#[test]
fn test_complete_encode_decode_cycle() {
    // 1. Create test file
    let test_data = b"This is a test file for V-Shield encoding/decoding";
    fs::write("test_input.bin", test_data)?;
    
    // 2. Encode to frames
    let encoder = Encoder::new(EncoderConfig {
        input_file: "test_input.bin".to_string(),
        output_file: "test_frames".to_string(),
        block_size: 8,
        redundancy_percent: 25,
        ..Default::default()
    });
    let encoded = encoder.encode()?;
    
    // Save frames
    encoded.save_as_images("test_frames")?;
    
    // 3. Decode from frames
    let decoder = Decoder::new(DecoderConfig {
        input_frames_dir: "test_frames".to_string(),
        output_file: "test_output.bin".to_string(),
        token: encoded.token.clone(),
    });
    let decoded = decoder.decode()?;
    decoded.save("test_output.bin")?;
    
    // 4. Verify
    let original = fs::read("test_input.bin")?;
    let recovered = fs::read("test_output.bin")?;
    assert_eq!(original, recovered);
    
    // 5. Cleanup
    fs::remove_file("test_input.bin")?;
    fs::remove_file("test_output.bin")?;
}
```

### Multi-Frame Encoding Test

```rust
#[test]
fn test_multiframe_encoding() {
    // Create file larger than single frame capacity
    // Frame capacity ≈ 4KB, so create 100KB file
    let mut large_data = Vec::new();
    for i in 0..25600 {
        large_data.extend_from_slice(&(i as u32).to_le_bytes());
    }
    
    let encoder = Encoder::new(config);
    let encoded = encoder.encode()?;
    
    // Should generate multiple frames
    assert!(encoded.num_frames > 1);
    
    // Each frame should have correct metadata
    assert!(encoded.frames[0].metadata.is_some()); // First
    assert!(encoded.frames[1].metadata.is_none()); // Others
}
```

### Error-Resilience Test

```rust
#[test]
fn test_decode_with_pixel_corruption() {
    // Encode small file
    let encoded = encode_test_file()?;
    encoded.save_as_images("test_frames")?;
    
    // Load frame PNG and corrupt some pixels
    let mut img = image::open("test_frames/frame_0000.png")?;
    let pixels = img.as_mut_rgb8().unwrap();
    
    // Corrupt ~5% of pixels
    let corruption_count = (pixels.len() as f32 * 0.05) as usize;
    for i in 0..corruption_count {
        let idx = (i * 179) % pixels.len(); // Pseudo-random spread
        pixels[idx] = 255 ^ pixels[idx]; // Flip bits
    }
    img.save("test_frames/frame_0000_corrupted.png")?;
    
    // Decode - should still work with error correction
    let decoder = Decoder::new(config);
    let decoded = decoder.decode()?;
    
    // Verify correctness
    assert_eq!(decoded.data, original_data);
}
```

---

## YouTube Compatibility Tests (Phase 1.5)

These tests involve real YouTube uploads and downloads.

### Test Environment Setup

```bash
mkdir -p tests/youtube_testbed
cd tests/youtube_testbed

# Create test files of various sizes
dd if=/dev/urandom of=test_100kb.bin bs=1K count=100
dd if=/dev/urandom of=test_1mb.bin bs=1M count=1
dd if=/dev/urandom of=test_10mb.bin bs=1M count=10
```

### Step 1: Upload Test Videos

```python
#!/usr/bin/env python3
import subprocess
import os
from datetime import datetime

test_files = [
    ("test_100kb.bin", "8x8"),
    ("test_100kb.bin", "16x16"),
    ("test_1mb.bin", "8x8"),
]

for filename, block_size in test_files:
    # Encode
    os.system(f"""
        vshield-encode \
        --input {filename} \
        --output frames_{block_size} \
        --block-size {block_size.split('x')[0]}
    """)
    
    # Create video
    os.system(f"""
        ffmpeg -framerate 30 \
        -i frames_{block_size}/frame_%04d.png \
        -c:v libx264 -pix_fmt yuv420p \
        -b:v 5000k \
        test_{filename.split('.')[0]}_bs{block_size}.mp4
    """)
    
    # TODO: Upload to YouTube (Unlisted) using YouTube API
    # Store video_id and metadata for testing
```

### Step 2: Download at Multiple Qualities

```bash
#!/bin/bash

# Download 360p
youtube-dl -f 18 "$VIDEO_URL" -o "test_360p.mp4"

# Download 720p
youtube-dl -f 22 "$VIDEO_URL" -o "test_720p.mp4"

# Download 1080p
youtube-dl -f 137+140 "$VIDEO_URL" -o "test_1080p.mp4"

# Extract frames
for quality in 360p 720p 1080p; do
    ffmpeg -i "test_${quality}.mp4" \
           "frames_yt_${quality}/frame_%04d.png"
done
```

### Step 3: Analyze Compression Artifacts

```rust
#[test]
fn test_youtube_compression_analysis() {
    // Compare original frames with YouTube-downloaded frames
    
    // Load original
    let original = image::open("frames_8x8/frame_0000.png")?;
    
    // Load YouTube 1080p
    let yt_1080 = image::open("frames_yt_1080p/frame_0000.png")?;
    
    // Load YouTube 720p
    let yt_720 = image::open("frames_yt_720p/frame_0000.png")?;
    
    // Load YouTube 360p
    let yt_360 = image::open("frames_yt_360p/frame_0000.png")?;
    
    // Calculate PSNR (Peak Signal-to-Noise Ratio)
    let psnr_1080 = calculate_psnr(&original, &yt_1080);
    let psnr_720 = calculate_psnr(&original, &yt_720);
    let psnr_360 = calculate_psnr(&original, &yt_360);
    
    println!("PSNR Analysis:");
    println!("  1080p: {:.2} dB", psnr_1080);
    println!("  720p:  {:.2} dB", psnr_720);
    println!("  360p:  {:.2} dB", psnr_360);
    
    // PSNR > 30 dB is considered "good quality"
    assert!(psnr_1080 > 28.0); // YouTube compression is aggressive
    assert!(psnr_720 > 26.0);
    assert!(psnr_360 > 24.0);
}
```

### Step 4: Test Decoding from YouTube Downloads

```rust
#[test]
fn test_decode_youtube_360p_quality() {
    // Try decoding video downloaded at 360p quality
    let decoder = Decoder::new(DecoderConfig {
        input_frames_dir: "frames_yt_360p".to_string(),
        output_file: "recovered_360p.bin".to_string(),
        token: "vshield://...".to_string(),
    });
    
    let result = decoder.decode()?;
    
    // Verify recovered data matches original
    let original = fs::read("test_100kb.bin")?;
    let recovered = fs::read("recovered_360p.bin")?;
    
    assert_eq!(original, recovered);
}
```

### Step 5: Measure Failure Rates

```rust
fn measure_compression_loss() {
    for quality in &["360p", "720p", "1080p"] {
        for block_size in &[4, 8, 16] {
            println!("Testing block size {}x{} at {}...", block_size, block_size, quality);
            
            match test_decode_youtube(block_size, quality) {
                Ok(data) => println!("  ✓ Success - 0% loss"),
                Err(e) => {
                    // Count how many bytes were recoverable
                    let recovery_rate = calculate_recovery_rate(&e);
                    println!("  ✗ Failed - {:.1}% loss", (1.0 - recovery_rate) * 100.0);
                }
            }
        }
    }
}
```

---

## Performance Benchmarking

Create: `benches/encoding_speed.rs`

```rust
use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn bench_encoding(c: &mut Criterion) {
    c.bench_function("encode_100kb", |b| {
        b.iter(|| {
            let encoder = Encoder::new(black_box(config));
            encoder.encode().unwrap()
        })
    });
    
    c.bench_function("encode_1mb", |b| {
        b.iter(|| {
            let encoder = Encoder::new(black_box(large_config));
            encoder.encode().unwrap()
        })
    });
}

fn bench_decoding(c: &mut Criterion) {
    c.bench_function("decode_100kb", |b| {
        b.iter(|| {
            let decoder = Decoder::new(black_box(config));
            decoder.decode().unwrap()
        })
    });
}

criterion_group!(benches, bench_encoding, bench_decoding);
criterion_main!(benches);
```

Run benchmarks:
```bash
cargo bench
```

---

## Continuous Integration Testing

### GitHub Actions Workflow

Create: `.github/workflows/test.yml`

```yaml
name: Tests

on: [push, pull_request]

jobs:
  test:
    runs-on: ${{ matrix.os }}
    strategy:
      matrix:
        os: [ubuntu-latest, windows-latest, macos-latest]
        rust: [stable, nightly]
    
    steps:
      - uses: actions/checkout@v3
      - uses: actions-rs/toolchain@v1
        with:
          toolchain: ${{ matrix.rust }}
      - name: Run tests
        run: cargo test --all
      - name: Run clippy
        run: cargo clippy -- -D warnings
      - name: Format check
        run: cargo fmt -- --check
```

---

## Test Coverage Targets

| Component | Coverage Target | Status |
|-----------|------------------|--------|
| Core Protocol | 90% | 🟡 In Progress |
| Encoder | 85% | 🟡 In Progress |
| Decoder | 80% | 🟡 In Progress |
| ECC | 95% | 🟡 In Progress |
| Crypto | 100% | 🟡 In Progress |

---

## Testing Checklist

Before releasing a new version:

- [ ] All unit tests pass
- [ ] All integration tests pass
- [ ] No clippy warnings
- [ ] Code formatted (rustfmt)
- [ ] Documentation updated
- [ ] Performance benchmarks run
- [ ] YouTube testbed passes
- [ ] Manual functional testing done

---

## Debugging Failed Tests

### Enable Verbose Logging

```bash
RUST_LOG=debug cargo test -- --nocapture
```

### Run Single Test

```bash
cargo test test_frame_header_serialization -- --nocapture
```

### Save Test Artifacts

```rust
#[test]
fn test_with_debug_save() {
    let result = encode(...)?;
    
    // Save for manual inspection
    result.save_as_images("test_output_debug")?;
    
    // Will persist in target/debug/test_output_debug/
}
```

---

## Troubleshooting

### "Token mismatch after decoding"
- Check that encryption/decryption nonce is consistent
- Verify token derivation logic is identical in encoder/decoder

### "Anchor detection failed"
- Ensure minimum anchor size survives downsampling
- Check pattern matching algorithm against 1:1:3:1:1 ratio

### "YouTube decode fails at 360p"
- Increase ECC redundancy
- Try larger block sizes (16x16 vs 8x8)
- Verify chroma subsampling doesn't destroy critical data

---

---

**Happy Testing! 🧪**

Report test failures with reproduction steps on GitHub Issues.

