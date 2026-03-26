# YouTube Compression Analysis & V-Shield Compatibility

## Overview

YouTube applies sophisticated compression to all uploaded videos. V-Shield is specifically designed to survive this compression and remain decodable. This document explains:

1. **How YouTube compresses videos**
2. **Why traditional steganography fails**
3. **How V-Shield survives compression**
4. **Testing requirements per quality level**

---

## YouTube Video Processing Pipeline

### Step 1: Ingest & Validation
- Accepts: MP4, WebM, AVI, MOV, FLV, MPEG-4, etc.
- Resolution range: 256×144 (min) to 7680×4320 (8K)
- Frame rate: 1 FPS to 60 FPS
- Maximum file size: 256 GB

### Step 2: Transcoding

YouTube transcodes to **multiple quality tiers**:

| Tier | Resolution | Bitrate | Codec | Profile |
|------|-----------|---------|-------|---------|
| 360p | 640×360 | 1,000 kbps | H.264 | Baseline |
| 480p | 854×480 | 2,500 kbps | H.264 | Main |
| 720p | 1280×720 | 5,000 kbps | H.264 / VP9 | High |
| 1080p | 1920×1080 | 8,000 kbps | H.264 / VP9 | High |
| 1440p | 2560×1440 | 16,000 kbps | VP9 | High 10 |
| 2160p (4K) | 3840×2160 | 45,000 kbps | VP9 | High 10 |

### Step 3: Color Space Processing

YouTube **always uses YUV 4:2:0 chroma subsampling**:

```
Original RGB (1920×1080):
Y channel:  1920×1080 (full resolution)
U channel:  1920×1080 → 960×540 (❌ Half resolution)
V channel:  1920×1080 → 960×540 (❌ Half resolution)
```

**Why:** Human eyes are more sensitive to luminance (brightness) than chroma (color). This saves 50% bandwidth without noticeable quality loss.

### Step 4: Macroblock Encoding

H.264 divides video into **16×16 macroblock** structures:
- Each block is one coding unit (CU)
- Motion vectors reference previous/future frames
- DCT coefficients coded with Huffman entropy encoding
- Quality reduction happens per-block based on quantization

---

## Why Standard Steganography Fails

### Traditional LSB (Least Significant Bit) Hiding

```
Original pixel: 255 (11111111 binary)
Hidden data:    1 (bit in LSB)
Result:         254 (11111110 binary)  ← Changed!

YouTube compression:
  H.264 encoding → DCT transform
  Quantization levels rounded
  IDCT inverse transform
  Result: Could be 253, 254, 255, or 256!
  ❌ Hidden bit is lost
```

### Why:
- **DCT coefficients are floating-point**
- **Quantization rounds values** (especially at low bitrates)
- **Rounding is unpredictable** (depends on I/P/B frame type)
- **Small changes get eliminated** as "noise"

---

## How V-Shield Survives

### 1: Block-Level Encoding (NOT Pixel-Level)

Instead of hiding 1 bit per pixel, V-Shield hides **3 bits per 8×8 block**:

```
V-Shield Block (8×8 pixels = 64 pixels):
┌─────────────────┐
│  XXXXXXXX       │  All 64 pixels in block
│  XXXXXXXX       │  set to SAME COLOR
│  XXXXXXXX       │
│  XXXXXXXX       │  Only 1 color per block
│  XXXXXXXX       │  = 1 symbol = 3 bits
│  XXXXXXXX       │
│  XXXXXXXX       │
│  XXXXXXXX       │
└─────────────────┘

8 colors available (2³ = 3 bits):
- Black (000)
- Dark Gray (001)
- Gray (010)
- Light Gray (011)
- White (100)
- Dark Red (101)
- Dark Blue (110)
- Dark Green (111)
```

### 2: Color Space Optimization

V-Shield uses **high-contrast luminance differences**:

```
Color Palette in YUV:
Black   → Y=16  (very dark)
White   → Y=235 (very bright)
Gray    → Y=128 (middle)

Difference (White - Black) = 219 levels
Even aggressive quantization preserves major luminance shifts
```

### 3: Interleaving Across Frame

Instead of sequential placement:

**❌ Bad (loses whole packet if one region compressed):**
```
Frame 1920×1080:
┌────────────────────────────────────┐
│ Packet1 Packet1 Packet2 Packet2 ... │
│ Packet1 Packet1 Packet2 Packet2 ... │
│ Packet1 Packet1 Packet2 Packet2 ... │  
│   ↑ If this area is heavily compressed, Packet1 is lost
└────────────────────────────────────┘
```

**✓ Good (spreads across whole frame):**
```
Frame 1920×1080:
┌────────────────────────────────────┐
│ P1 P100 P2 P99 P3 P98 P4 P97 ...  │
│ P50 P51 P52 P1 ... P200 P201 P202 │
│ P150 P151 P3 P152 P153 P4 P154 ... │
│ Scattered throughout → If any area is compressed,
│ only 1-2 blocks per packet affected
└────────────────────────────────────┘
```

### 4: Reed-Solomon Error Correction

With 25% redundancy on 250 data blocks = 62.5 ECC blocks:

```
250 data symbols + 62 ECC symbols = 312 total

Reed-Solomon can correct up to ⌊ECC_symbols/2⌋ errors:
Correction capacity = 31 symbol errors

Even if YouTube corrupts entire 16×16 macroblock:
= ~4 (8×8) blocks harmed
= Still well within correction capacity
```

### 5: Scale-Invariant Anchors (Finder Patterns)

Like QR codes, V-Shield uses **proportional patterns**, not pixel-count patterns:

```
Standard (❌ fails on downscale):
Original:  ████ ████ ████ ████  (20 pixels wide)
Downscale: ██ ██ ██ ██           (10 pixels, pattern lost!)

V-Shield (✓ survives downscale):
Ratio pattern: 1:1:3:1:1
Original:  ███████████████████████  (24 units)
Downscale: ██████████████           (12 units)
Ratio preserved!

Detection algorithm:
FOR EACH one-pixel-wide line:
    IF pattern matches 1:1:3:1:1 ratio
        THEN found an anchor edge
```

---

## Compression Loss Analysis

### H.264 Quantization Impact

H.264 uses quantization parameter (QP):

| QP | Bitrate | Data Loss |
|----|---------|-----------|
| 28 | 8,000 kbps | ~2% |
| 32 | 5,000 kbps | ~5% |
| 36 | 2,500 kbps | ~12% |
| 40 | 1,000 kbps | ~25% |

**V-Shield targets:**
- 25% redundancy at QP=32 (720p typical)
- 30% redundancy at QP=36 (480p typical)
- ~50% redundancy at QP=40 (360p extreme)

### Chroma Subsampling Impact

```
1920×1080 with 8×8 blocks = 240 × 135 = 32,400 blocks

YUV 4:2:0 subsampling means:
- Y-channel blocks: 32,400 (preserved)
- U/V-channel info: Only ~100 reference points

V-Shield's Solution:
1. Store critical metadata in Y-channel ONLY
2. Use colors with different Y-values
3. Spread data across entire frame
   → Chroma loss affects <2% of information
```

---

## Testing at Each Quality Level

### 360p Quality Testing

**Challenges:**
- Most aggressive compression (QP ≈ 40)
- Extreme chroma subsampling effects
- Motion compensation may be inaccurate for synthetic patterns

**V-Shield Requirements:**
- Block size: 16×16 (larger than macroblock 16×16)
- Redundancy: 35-40%
- Test with high-motion reference videos (make sure it's not just pattern detection)

**Test Steps:**
1. Encode test file with 16×16 blocks, 40% redundancy
2. Upload to YouTube
3. Download 360p version
4. Attempt decode - should succeed 100%

### 480p Quality Testing

**Challenges:**
- Balanced compression (QP ≈ 36)
- Macroblock artifacts visible but less extreme
- Some data loss expected but recoverable

**V-Shield Requirements:**
- Block size: 8×8 (good balance)
- Redundancy: 30%

**Test Steps:**
1. Encode with 8×8 blocks, 30% redundancy
2. Upload to YouTube
3. Download 480p version
4. Attempt decode - should succeed 100%

### 720p Quality Testing

**Challenges:**
- Good quality (QP ≈ 32)
- Block artifacts minimal
- Most reliable decoding

**V-Shield Requirements:**
- Block size: 8×8
- Redundancy: 25% (sufficient)

### 1080p Quality Testing

**Challenges:**
- Excellent quality (QP ≈ 28)
- Least artifact
- Most data preserved

**V-Shield Requirements:**
- Block size: 4×4 (maximum density)
- Redundancy: 20% (minimal, but sufficient)

---

## Expected Loss Rate Estimates (Phase 1.5 Testing Goals)

| Quality | Block Size | Redundancy | Expected Loss | Decode Success |
|---------|-----------|-----------|---------------|-----------------|
| 360p | 16×16 | 40% | 5-8 blocks | ✓ Yes |
| 480p | 8×8 | 30% | 8-12 blocks | ✓ Yes |
| 720p | 8×8 | 25% | 5-8 blocks | ✓ Yes |
| 1080p | 4×4 | 20% | 2-4 blocks | ✓ Yes |

---

## Practical YouTube Testing Protocol

### Equipment Needed
- YouTube account (upload permissions)
- Video download tool (youtube-dl, yt-dlp)
- V-Shield encoder/decoder
- FFmpeg (for frame extraction)
- Python script (to analyze PSNR/SSIM)

### Test Procedure

**Day 1: Setup**
```bash
# Create test files
echo "Binary test data" > test_100kb.bin
dd if=/dev/urandom of=test_1mb.bin bs=1M count=1

# Encode at different block sizes
vshield-encode --input test_100kb.bin --output frames_8x8 --block-size 8
vshield-encode --input test_100kb.bin --output frames_16x16 --block-size 16

# Convert to MP4
ffmpeg -framerate 30 -i frames_8x8/frame_%04d.png -c:v libx264 test_8x8.mp4
ffmpeg -framerate 30 -i frames_16x16/frame_%04d.png -c:v libx264 test_16x16.mp4
```

**Day 2: Upload**
```bash
# Upload as Unlisted videos
# Use YouTube API or Web UI
# Note video IDs
```

**Day 3-5: Download & Test**
```bash
# Download each quality
yt-dlp -f 18 https://youtube.com/watch?v=... -o test_360p.mp4
yt-dlp -f 22 https://youtube.com/watch?v=... -o test_720p.mp4
yt-dlp -f 137+140 https://youtube.com/watch?v=... -o test_1080p.mp4

# Extract frames from downloaded videos
ffmpeg -i test_360p.mp4 frames_360p/frame_%04d.png
ffmpeg -i test_720p.mp4 frames_720p/frame_%04d.png
ffmpeg -i test_1080p.mp4 frames_1080p/frame_%04d.png

# Attempt decode
vshield-decode --input frames_360p --output recovered_360p.bin --token "vshield://..."
vshield-decode --input frames_720p --output recovered_720p.bin --token "vshield://..."

# Verify
sha256sum test_100kb.bin recovered_360p.bin  # Should match!
```

### Analysis Script

```python
#!/usr/bin/env python3
import cv2
import numpy as np
from pathlib import Path

def calculate_psnr(frame1, frame2):
    """Peak Signal-to-Noise Ratio"""
    mse = np.mean((frame1.astype(float) - frame2.astype(float)) ** 2)
    if mse == 0:
        return 100
    return 10 * np.log10(255.0 ** 2 / mse)

# Compare original vs YouTube-degraded
original = cv2.imread("frames_8x8/frame_0000.png")
downloaded_360p = cv2.imread("frames_yt_360p/frame_0000.png")

# Resize for comparison (different resolutions)
h360 = cv2.resize(downloaded_360p, (original.shape[1], original.shape[0]))

psnr = calculate_psnr(original, h360)
print(f"PSNR (360p): {psnr:.2f} dB")
print(f"Quality: {'Excellent' if psnr > 30 else 'Good' if psnr > 26 else 'Fair' if psnr > 20 else 'Poor'}")
```

---

## Red Flags (When Decoding Fails)

| Symptom | Likely Cause | Fix |
|---------|-------------|-----|
| 0 frames detected | Anchor pattern not found | Increase block size to 16×16 |
| Random data in output | Encryption key mismatch | Verify token is correct |
| Partial recovery only | ECC insufficient | Increase redundancy to 30-40% |
| All blocks corrupted | Bitrate too low for quality | Use higher quality (720p instead of 360p) |
| Hash mismatch | Data corruption | Increase ECC, test with 1080p first |

---

## Phase 1.5 Success Criteria

✓ All qualities (360p, 480p, 720p, 1080p) decode successfully  
✓ Small files (<1MB) achieve 100% recovery rate  
✓ Anchor detection works after platform re-compression  
✓ Token verification prevents tampering  
✓ Performance is acceptable (<5 min per file)  

If any fails:
- Adjust block size (larger = more robust)
- Increase ECC redundancy (slower encoding, better recovery)
- Document findings for Phase 2 optimization

---

## Advanced: VP9 Codec Considerations

YouTube uses VP9 for 720p+ on some systems. VP9 characteristics:

**Advantages:**
- Better temporal compression (video quality often better than H.264)
- Smaller file sizes

**Challenges:**
- More aggressive motion compensation
- Adaptive quantization per region
- Tiling (frame divided into independent regions)

**V-Shield Strategy:**
- Interleaving already handles regional variation
- Block size >= 16×16 unaffected by tiling
- Test with both H.264 and VP9 videos

---

---

**Phase 1.5 is critical!** Real YouTube testing will reveal any issues that local testing missed.

Start testing as soon as Phase 1 encoding is complete. 📹🔍

