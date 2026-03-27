# V-Shield: YouTube-Resistant Data Encoding System

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/Rust-1.70+-orange.svg)](https://www.rust-lang.org/)
[![Status: Alpha](https://img.shields.io/badge/Status-Alpha%20v0.1-blue)](docs/ROADMAP.md)

> Transform any digital file into visually-encoded video that survives YouTube's compression algorithms, recoverable with a cryptographic token.

## 🎯 Project Overview

**V-Shield** is a cryptographic steganography system designed to hide arbitrary data inside video frames in a way that survives video platform compression (particularly YouTube's H.264 codec). The system is designed around three independent modules that work together:

1. **Desktop Encoder** - Converts files to encoded video frames (Tauri app + Rust)
2. **Core Engine** - Handles all encoding/decoding/crypto (Rust, compiles to WASM)
3. **Browser Extension** - Extracts files from YouTube videos in real-time (TypeScript + WASM)

## ⚠️ IMPORTANT: Read the Disclaimers

**Before using V-Shield, you MUST read:**
- [`DISCLAIMER.md`](DISCLAIMER.md) - Legal disclaimer and liability
- [`USER_RESPONSIBILITY.md`](USER_RESPONSIBILITY.md) - Your responsibilities as a user

**TL;DR:**
- You are 100% responsible for content you encrypt
- Tokens cannot be recovered if lost
- You must comply with all platform ToS and local laws
- This tool is neutral technology—its use is on you

---

## 🏗️ Architecture

### System Overview

```
┌─────────────────────────────────────────────────────────────┐
│                     V-Shield System                         │
├─────────────────────────────────────────────────────────────┤
│                                                             │
│  Creator Side               Core Engine       Viewer Side   │
│  ┌──────────────┐          ┌──────────┐     ┌───────────┐   │
│  │   Original   │          │          │     │ Browser   │   │
│  │    File      │────────→ │ Encoder  │────→│ Captures  │   │
│  └──────────────┘          │ Crypto   │     │ Video     │   │
│         ▲                  │ ECC      │     └─────┬─────┘   │
│         │                  │ Layout   │           │         │
│  ┌──────┴──────┐           └──────────┘           │         │
│  │   Token     │                                  ▼         │
│  │ (Keep Safe!)│          ┌──────────┐     ┌───────────┐    │
│  └─────────────┘          │          │     │  WASM     │    │
│         ▲                 │ Decoder  │────→│ Decoder   │    │
│         │                 │ Crypto   │     │ Extracts  │    │
│  ┌──────┴──────┐          │ ECC      │     └───────────┘    │
│  │  Re-encode  │          │ Layout   │                      │
│  │ if lost     │          └──────────┘                      │
│  └─────────────┘                                            │
│                                                             │
└─────────────────────────────────────────────────────────────┘
```

### Frame Protocol: The Foundation

Each video frame has a strict structure designed to survive YouTube compression:

```
┌──────────────────────────────────────────┐
│  Finder Patterns (Anchors) - 4 corners   │  ← QR-code style
│  (Scale-invariant 1:1:3:1:1 patterns)    │
├──────────────────────────────────────────┤
│  Frame Header                            │  ← Metadata about frame
│  [ID: u32] [BlockSize: u8] [Flags: u8]   │
├──────────────────────────────────────────┤
│  Metadata Block (First Frame Only)       │  ← File info + token
│  [Filename] [Size] [SHA256 Hash] [Token] │
├──────────────────────────────────────────┤
│  Interleaved Payload                     │  ← Data spread across
│  [8-color encoded blocks, scattered]     │     whole frame
├──────────────────────────────────────────┤
│  Reed-Solomon ECC (20-30% redundancy)    │  ← Error correction
│  Spread across frame                     │
└──────────────────────────────────────────┘
```

### Key Technologies

| Component | Technology | Purpose |
|-----------|-----------|---------|
| **Encoding/Decoding** | Rust | Performance + Memory Safety |
| **WASM Compilation** | wasm-bindgen | Browser compatibility |
| **Cryptography** | ChaCha20-Poly1305 | Fast, WASM-friendly encryption |
| **Error Correction** | Reed-Solomon (rs-erasure) | YouTube compression resilience |
| **Video Rendering** | FFmpeg (external) | MP4 generation |
| **Video Capture** | Canvas API | Browser frame extraction |
| **Desktop UI** | Tauri + React/Svelte | Native app with web UI |

---

## 📦 Phase Breakdown

### Phase 1: Foundation (Current - Alpha)
- [x] Frame protocol definition
- [x] Finder pattern generation  
- [x] Interleaving system
- [x] Reed-Solomon ECC integration
- [x] Encoder pipeline (CLI)
- [x] Decoder pipeline (CLI)
- [ ] Integration tests
- [ ] YouTube testbed (Upload real test video)

### Phase 1.5: YouTube Compatibility Testing
- [ ] Test video upload pipeline
- [ ] Download multiple quality levels
- [ ] Measure compression losses
- [ ] Refine ECC parameters
- [ ] Determine optimal block size

### Phase 2: Color Optimization  
- [ ] YUV color space analysis
- [ ] Increased data density (8 colors)
- [ ] Better compression resilience
- [ ] Performance optimization

### Phase 3: Browser Extension
- [ ] Compile decoder to WASM
- [ ] JavaScript/TypeScript wrapper
- [ ] YouTube video capture
- [ ] Chrome extension manifest
- [ ] Token UI and management

### Phase 4: Streaming "Video in Video"
- [ ] MediaSource Extensions (MSE)
- [ ] Real-time streaming decode
- [ ] Overlay video rendering
- [ ] Network optimization

---

## 🚀 Quick Start

### Prerequisites
- Rust 1.70+
- FFmpeg (for video rendering)
- Node.js 16+ (for extension development)

### Installation

```bash
# Clone repository
git clone https://github.com/NiZaMinius/v-shield
cd v-shield

# Build encoder
cd crates/vshield-enc
cargo build --release

# Build decoder
cd ../vshield-dec
cargo build --release
```

### Usage: Encoder

```bash
# Encode file to video frames
./target/release/vshield-encode \
  --input myfile.bin \
  --output frames/ \
  --block-size 8 \
  --redundancy 25

# Output includes:
# - frames/frame_0000.png ... frame_XXXX.png (1920x1080)
# - frames/metadata.json (contains token and file hash)
```

### Usage: Decoder

```bash
# Decode video frames back to original file
./target/release/vshield-decode \
  --input frames/ \
  --output recovered.bin \
  --token "vshield://..."

# Optionally verify:
# sha256sum recovered.bin  # should match original
```

### Video Pipeline (Future)

```bash
# Convert frames to MP4 (using FFmpeg)
ffmpeg -framerate 30 -i frames/frame_%04d.png \
  -c:v libx264 -pix_fmt yuv420p \
  -b:v 5000k output.mp4

# Upload output.mp4 to YouTube
```

---

## 🔐 Security Model

### Threat Model

**Protected Against:**
- ✓ YouTube compression artifacts (H.264/VP9)
- ✓ Chroma subsampling (4:2:0)
- ✓ Bitrate reduction
- ✓ Quality degradation (360p, 480p, 720p, 1080p)
- ✓ Platform modification (watermarks, logos)

**NOT Protected Against:**
- ✗ Strategic removal (YouTube moderators)
- ✗ Cryptanalysis (with sufficient resources)
- ✗ Brute-force attacks (use strong tokens)
- ✗ Man-in-the-middle (use HTTPS for distribution)

### Cryptography

**Encryption:**
- **Algorithm:** ChaCha20-Poly1305 (AEAD)
- **Key Size:** 256 bits (derived from token)
- **Nonce:** Fixed (improvement for v0.2: randomized)
- **Authentication:** Poly1305 MAC

**Token Format:**
```
vshield://{UUID}

Example: vshield://550e8400-e29b-41d4-a716-446655440000
```

**Recovery:**
- Token saved in `metadata.json` in frames directory
- Token derivable from generator (same exact file → different token)
- **No recovery service.** Don't lose the token.

---

## 🎨 Color Palette

### Current (Phase 1): Grayscale + Primary Colors

| Color | RGB | YUV | Bits/Block |
|-------|-----|-----|-----------|
| Black | (0, 0, 0) | (16, 128, 128) | 000 |
| DarkGray | (64, 64, 64) | (69, 128, 128) | 001 |
| Gray | (128, 128, 128) | (128, 128, 128) | 010 |
| LightGray | (192, 192, 192) | (183, 128, 128) | 011 |
| White | (255, 255, 255) | (235, 128, 128) | 100 |
| DarkRed | (128, 0, 0) | (54, 21, 192) | 101 |
| DarkBlue | (0, 0, 128) | (30, 145, 54) | 110 |
| DarkGreen | (0, 128, 0) | (107, 52, 47) | 111 |

**Why these colors?**
- Designed for chroma subsampling (Y-channel is most important)
- High contrast to survive aggressive compression
- Limited palette to avoid JPG banding

### Future (Phase 2): YUV Optimization
- More colors optimized for YUV 4:2:0 subsampling
- Different luminance levels for better robustness

---

## 🧪 Testing

### Unit Tests
```bash
cargo test --lib
```

### Integration Tests
```bash
cargo test --test '*'
```

### YouTube Real-World Test (Phase 1.5)
1. Encode a small test file
2. Convert frames to MP4  
3. Upload as Unlisted video
4. Download at multiple qualities (360p, 720p, 1080p)
5. Verify decoding works
6. Measure compression artifacts

---

## 📊 Performance Targets

### Encoding
- **Small files** (< 1MB): < 5 seconds
- **Medium files** (1-100MB): < 1 minute
- **Output frame rate:** 30 FPS (compatible with YouTube)

### Decoding
- **Real-time:** 30 FPS @ 1080p
- **Streaming:** Start playback within 2-3 seconds
- **WASM decoder:** < 5MB total size

### Data Density
- **Phase 1:** ~0.5 bytes per block (3 bits/block)
- **Phase 2:** ~1 byte per block (8 bits/block)
- **1920x1080 @ 8px blocks:** 51,200 blocks per frame
  - Phase 1: ~25 KB per frame
  - Phase 2: ~50 KB per frame

---

## 🛠️ Development Roadmap

### v0.1.0 (Alpha - Current)
- [x] Core Rust library
- [x] CLI encoder/decoder
- [ ] Basic testing

### v0.2.0 (Beta - Q2 2026)
- [ ] Tauri desktop app
- [ ] FFmpeg integration
- [ ] Browser extension (WASM)
- [ ] Real YouTube testing

### v0.3.0 (Release Candidate)
- [ ] Streaming video playback
- [ ] Token recovery UI
- [ ] Advanced color optimization
- [ ] WASM optimization

### v1.0.0 (Production)
- [ ] All of above
- [ ] Security audit
- [ ] Performance tuning
- [ ] Production-ready documentation

---

## 🤝 Contributing

We welcome contributions! But please read our guidelines:

1. **Fork and branch** off `develop`
2. **Write tests** for new features
3. **Follow Rust conventions** (rustfmt, clippy)
4. **Document your changes**
5. **Submit PR** with description

### Code of Conduct

- Respect others
- No harassment
- Assume good faith
- Help each other

---

## 📚 Documentation

- [`ARCHITECTURE.md`](docs/ARCHITECTURE.md) - Deep dive into system design
- [`PROTOCOL.md`](docs/PROTOCOL.md) - Frame protocol specification
- [`YOUTUBE_COMPATIBILITY.md`](docs/YOUTUBE_COMPATIBILITY.md) - Compression analysis
- [`API.md`](docs/API.md) - Rust API documentation
- [`ROADMAP.md`](docs/ROADMAP.md) - Detailed project roadmap

---

## ⚖️ Legal

**Please read:**
- [`DISCLAIMER.md`](legal/DISCLAIMER.md) - What we do/don't handle
- [`USER_RESPONSIBILITY.md`](legal/USER_RESPONSIBILITY.md) - Your obligations

### License
MIT License - See [`LICENSE`](LICENSE) for details

### No Warranty
THE SOFTWARE IS PROVIDED "AS-IS" WITHOUT WARRANTY OF ANY KIND.

---

## 🔗 Resources

- **GitHub Issues:** Report bugs and suggest features
- **Discussions:** Architecture questions, design decisions
- **Wiki:** Community knowledge base
- **YouTube Channel:** (Coming soon) Testing & demo videos

---

## 💭 FAQ

### Q: Can I recover my token if I lose it?
**A:** No. There is no recovery mechanism.

### Q: Will YouTube ban me?
**A:** If you follow YouTube ToS, no. If you hide violating content, yes.

### Q: Can I hide copyrighted content?
**A:** No. Encoding doesn't make copyright infringement legal.

### Q: Isn't this illegal?
**A:** The tool is neutral. Its legality depends on how YOU use it.

### Q: How do I back up my token?
**A:** Save `metadata.json` in multiple secure locations.

### Q: What if YouTube's compression destroys my video?
**A:** The system is designed to survive, but no guarantee. Always test first.

---

## 📞 Support

- **Technical Issues:** GitHub Issues
- **Security Vulnerabilities:** Email (responsible disclosure)
- **Legal Questions:** Consult a lawyer

---

## 🌟 Acknowledgments

- **Reed-Solomon ECC:** rs-erasure crate maintainers
- **Rust Community:** For amazing ecosystem
- **YouTube:** For creating the compression problem we solve

---

---

**V-Shield: Encode Data. Share Securely. Survive Compression.**

*Alpha v0.1 - March 2026*

---

## Next Steps

- [ ] Read the disclaimers
- [ ] Review the architecture
- [ ] Build from source
- [ ] Run tests
- [ ] Try encoding a small file
- [ ] Help us improve!

---

Questions? Issues? Ideas? **Open an issue on GitHub!**

