# V-Shield Quick Start Guide

> Get V-Shield running in 15 minutes  
> **For full documentation**, see [README.md](README.md)  
> **For legal obligations**, read [DISCLAIMER.md](DISCLAIMER.md) & [USER_RESPONSIBILITY.md](USER_RESPONSIBILITY.md)

---

## Prerequisites (5 min)

### Install Rust

```bash
# macOS / Linux
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env

# Windows (PowerShell - as Administrator)
iwr -useb https://win.rustup.rs | iex
```

Verify:
```bash
rustc --version  # Should show 1.70+
cargo --version
```

### Install FFmpeg (Optional, needed for video generation)

```bash
# macOS
brew install ffmpeg

# Ubuntu/Debian
sudo apt-get install ffmpeg

# Windows (Chocolatey)
choco install ffmpeg

# Or download from https://ffmpeg.org/download.html
```

---

## Setup (3 min)

```bash
# Clone and enter project
git clone https://github.com/nizaminius/v-shield.git
cd v-shield

# Build everything
cargo build --release

# Binaries are now in target/release/
```

---

## Encode a File (2 min)

### Create a Test File

```bash
echo "This is my secret data!" > secret.txt
```

### Encode It

```bash
./target/release/vshield-encode \
  --input secret.txt \
  --output encoded_frames \
  --block-size 8 \
  --redundancy 25
```

### What You Get

```
encoded_frames/
├── frame_0000.png
├── frame_0001.png
├── ... more frames ...
└── metadata.json
```

**⚠️ SAVE THIS TOKEN** (from output):
```
vshield://550e8400-e29b-41d4-a716-446655440000
```

This token is needed to decode the file. If you lose it, **the data is unrecoverable**.

---

## Generate Video (Optional)

Convert frames to MP4:

```bash
ffmpeg -framerate 30 \
  -i encoded_frames/frame_%04d.png \
  -c:v libx264 -pix_fmt yuv420p \
  -b:v 5000k \
  output.mp4
```

**Upload `output.mp4` to YouTube** (Unlisted or Private)

---

## Decode a File (2 min)

### Extract Frames (If from YouTube)

1. Download video from YouTube using youtube-dl:
   ```bash
   youtube-dl -f 22 "https://youtube.com/watch?v=VIDEO_ID" -o video.mp4
   ```

2. Extract frames:
   ```bash
   mkdir downloaded_frames
   ffmpeg -i video.mp4 downloaded_frames/frame_%04d.png
   ```

### Decode

```bash
./target/release/vshield-decode \
  --input encoded_frames \
  --output recovered.txt \
  --token "vshield://550e8400-e29b-41d4-a716-446655440000"
```

### Verify

```bash
# Check the recovered file matches the original
cat recovered.txt
# Should show: "This is my secret data!"
```

---

## Complete Workflow Example

```bash
# 1. Create test data
echo "Hello, V-Shield!" > original.txt

# 2. Encode
./target/release/vshield-encode \
  --input original.txt \
  --output my_frames

# Save the token from the output!
TOKEN="vshield://..."  # Copy from output

# 3. Generate MP4
ffmpeg -framerate 30 \
  -i my_frames/frame_%04d.png \
  -c:v libx264 -pix_fmt yuv420p \
  my_video.mp4

# 4. Decode (to verify it works)
./target/release/vshield-decode \
  --input my_frames \
  --output recovered.txt \
  --token "$TOKEN"

# 5. Verify
diff original.txt recovered.txt
# Should show no output (files are identical)
```

---

## Common Commands

### Get Help

```bash
./target/release/vshield-encode --help
./target/release/vshield-decode --help
```

### Adjust Parameters

```bash
# Larger block size = more compression-resistant but lower data density
./target/release/vshield-encode --block-size 16

# Higher redundancy = better error correction but slower
./target/release/vshield-encode --redundancy 35

# Custom frame size (usually 1920x1080)
./target/release/vshield-encode --width 3840 --height 2160
```

### Custom Output Location

```bash
./target/release/vshield-encode \
  --input /path/to/file \
  --output /path/to/output/directory
```

---

## Testing Your Setup

### Run Unit Tests

```bash
cargo test --lib
```

Should show:
```
test result: ok. X passed; 0 failed
```

### Run Full Test Suite

```bash
cargo test --all
```

---

## Troubleshooting

### "Command not found: vshield-encode"

Make sure it's compiled:
```bash
cargo build --release

# Then use full path
./target/release/vshield-encode --input test.txt --output frames
```

### "FFmpeg not found"

Either:
1. Install FFmpeg (see prerequisites)
2. Specify path: `/usr/bin/ffmpeg` or `C:\ffmpeg\bin\ffmpeg.exe`

### "Token is missing"

The token is printed at the end of encoding. Look for:
```
✅ Encoding complete!
   Token: vshield://...
```

Save it immediately! There's no way to recover it if lost.

### "Hash mismatch" During Decoding

This means the data was corrupted. Try:
1. Your token might be wrong (paste carefully)
2. Frames might be damaged (check file integrity)
3. YouTube compression might be too aggressive (use higher block size)

---

## Next Steps

### 1. Read the Full Documentation

- **[README.md](README.md)** - Full project overview
- **[DISCLAIMER.md](DISCLAIMER.md)** - Legal stuff
- **[USER_RESPONSIBILITY.md](USER_RESPONSIBILITY.md)** - Your obligations

### 2. Test with YouTube

This is critical! Real YouTube compression is different from local testing:

```bash
# 1. Encode a test file
./target/release/vshield-encode --input test.bin --output test_frames

# 2. Save the token somewhere safe

# 3. Generate video
ffmpeg -framerate 30 -i test_frames/frame_%04d.png -c:v libx264 test.mp4

# 4. Upload to YouTube as UNLISTED video

# 5. Download it back
youtube-dl -f 22 "https://youtube.com/watch?v=VIDEO_ID" -o yt_video.mp4

# 6. Extract frames
ffmpeg -i yt_video.mp4 yt_frames/frame_%04d.png

# 7. Try to decode
./target/release/vshield-decode \
  --input yt_frames \
  --output recovered.bin \
  --token "vshield://..."

# 8. Check if it matches
sha256sum test.bin recovered.bin
```

If this works, V-Shield is ready! If not, see [YOUTUBE_COMPATIBILITY.md](docs/YOUTUBE_COMPATIBILITY.md).

### 3. Understand the Legal Stuff

**You MUST read:**
- ✅ DISCLAIMER.md (your liability)
- ✅ USER_RESPONSIBILITY.md (your obligations)

TL;DR:
- You own the content you encode
- You can't hide illegal stuff
- You're responsible for your tokens
- Lost token = permanently lost data
- Platform might ban you if you violate ToS

---

## File Size Expectations

For planning purposes:

| Original File Size | Encoded Frames |Frame Count |MP4 Size (@ 5Mbps)|
|--------------------|-------|------------|---------|
| 100 KB | ~400 KB | 15 | 5 MB |
| 1 MB | ~4 MB | 150 | 45 MB |
| 10 MB | ~40 MB | 1,500 | 450 MB |

---

## Performance Tips

### Faster Encoding
- Use larger block size (16x16 vs 8x8)
- Lower redundancy (20% vs 25%)
- Smaller frame size (1280x720 vs 1920x1080)

### Faster Decoding
- Same strategies as above
- More RAM helps with frame processing

### Better YouTube Compatibility
- Use smaller block size (8x8 vs 16x16)
- Higher redundancy (30% vs 25%)
- Full resolution (1920x1080)
- Test with multiple qualities (360p, 720p, 1080p)

---

## Security Tips

### Token Storage

```bash
# ✅ Good: Store in password manager
# Save token in 1Password, Bitwarden, etc.

# ✅ Good: Encrypted file
# Use GPG or similar encryption

# ❌ Bad: Plain text
# Don't put in unencrypted files

# ❌ Bad: Internet
# Don't email it or use unencrypted messaging
```

### Backup Strategy

```bash
# Best: Multiple secure locations
1. Password manager (encrypted cloud backup)
2. Encrypted USB drive (physical backup)
3. Paper backup in safe (for truly critical data)

# Check your backups work!
```

### Token Compromise

If someone gets your token:

1. **Delete the video from YouTube immediately**
2. **Claim the original data is gone**
3. **Re-encode the file with the same original data**
   - This generates a NEW token
   - The video will look completely different (different "glitch art")
4. **Upload the new version**
5. **Tell viewers about the new token**

---

## API Usage (For Programmers)

If you want to use V-Shield in your own Rust code:

```rust
use vshield_enc::{Encoder, EncoderConfig};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = EncoderConfig {
        input_file: "myfile.bin".to_string(),
        output_file: "output_frames".to_string(),
        block_size: 8,
        redundancy_percent: 25,
        frame_width: 1920,
        frame_height: 1080,
    };

    let encoder = Encoder::new(config);
    let output = encoder.encode()?;
    
    println!("Token: {}", output.token);
    println!("Frames: {}", output.num_frames);
    
    output.save_as_images("output_frames")?;
    
    Ok(())
}
```

See [README.md](README.md) for more API examples.

---

## Getting Help

| Problem | Solution |
|---------|----------|
| Build errors | Run `cargo clean && cargo build --release` |
| Token lost | No recovery. Plan better backups next time. |
| YouTube decode fails | See [YOUTUBE_COMPATIBILITY.md](docs/YOUTUBE_COMPATIBILITY.md) |
| Legal questions | Consult a lawyer, not us |
| Bug report | GitHub Issues with reproducible steps |
| Security issue | Responsible disclosure (don't post publicly) |

---

## Summary

That's it! You now have:

✅ Rust installed  
✅ V-Shield compiled  
✅ Encoder & Decoder working  
✅ Understanding of the workflow  
✅ Knowledge of security practices  

### What's Next?

1. Try encoding/decoding a small file
2. Test with YouTube (if you dare!)
3. Read the full documentation
4. Give feedback on GitHub

---

## Remember

> **⚠️ Lost Token = Lost Data. Forever.**  
> **⚠️ YouTube Can Ban Your Account.**  
> **⚠️ Read the Disclaimers.**  
> **✅ Back Up Your Tokens.**  
> **✅ Test Before Using.**  
> **✅ Follow Platform ToS.**  

---

**Welcome to V-Shield!** 🚀

Questions? [Open an issue on GitHub.](https://github.com/yourusername/v-shield/issues)

