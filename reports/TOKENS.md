# V-Shield Phase 1 - Encoding/Decoding Instructions

## ✅ Encoded Files Summary

All test files have been successfully encoded! 

### Phase 1 Status:
- ✅ **Encoding**: FULLY WORKING - Data encrypted, frames created, tokens generated
- 🟡 **Decoding**: Framework in place, will be completed in Phase 1.5
- 🟡 **Error Correction**: Framework implemented, full reconstruction in Phase 1.5

### Test Files Encoded:

### Test File 1: small.txt (50 bytes)
```
Input:  test_files/small.txt
Output: output/test_small/
Token:  vshield://5650f5a0-d8c8-4d0c-a00c-46124b47d394
```

To decode and recover the file:
```bash
./target/release/vshield-decode.exe \
  --input output/test_small \
  --output output/recovered_small.txt \
  --token "vshield://5650f5a0-d8c8-4d0c-a00c-46124b47d394"
```

Then verify:
```bash
# Windows: Compare files
fc test_files\small.txt output\recovered_small.txt

# Linux: Compare files  
cmp test_files/small.txt output/recovered_small.txt
```

---

### Test File 2: config_small.json (22 bytes)
```
Input:  test_files/config_small.json
Output: output/test_config/
Token:  vshield://9c83bf99-f811-459f-aee1-3cfddae52208
```

To decode:
```bash
./target/release/vshield-decode.exe \
  --input output/test_config \
  --output output/recovered_config.json \
  --token "vshield://9c83bf99-f811-459f-aee1-3cfddae52208"
```

---

### Test File 3: test_data.txt (25 bytes)
```
Input:  test_files/test_data.txt
Output: output/test_data/
Token:  vshield://00035e17-8302-46d1-a17a-12bb2bd00141
```

To decode:
```bash
./target/release/vshield-decode.exe \
  --input output/test_data \
  --output output/recovered_test_data.txt \
  --token "vshield://00035e17-8302-46d1-a17a-12bb2bd00141"
```

---

## 📋 Quick Reference Table

| Test | Input File | Bytes | Token | Status |
|------|-----------|-------|-------|--------|
| 1 | small.txt | 50 | `vshield://5650f5a0-d8c8-4d0c-a00c-46124b47d394` | ✅ Encoded |
| 2 | config_small.json | 22 | `vshield://9c83bf99-f811-459f-aee1-3cfddae52208` | ✅ Encoded |
| 3 | test_data.txt | 25 | `vshield://00035e17-8302-46d1-a17a-12bb2bd00141` | ✅ Encoded |

---

## 🎯 What's Happening

### Encoding (Already Done ✅)
1. Read input file
2. Calculate SHA-256 hash for integrity
3. Generate unique token (UUID-based)
4. Encrypt data with ChaCha20-Poly1305 (256-bit key derived from token)
5. Apply Reed-Solomon error correction (20% redundancy)
6. Render to PNG frame with:
   - Finder patterns (corners) for frame detection
   - 8-color palette optimized for YouTube's 4:2:0 chroma subsampling
   - Data embedded in 8×8 blocks
7. Save PNG + metadata.json

### Decoding (Next Step)
1. Load PNG frames
2. Detect frame bounds using finder patterns
3. Extract color data from blocks
4. Apply de-interleaving
5. Reconstruct with Reed-Solomon error correction
6. Decrypt using token
7. Verify SHA-256 hash
8. Save recovered file

---

## 🚀 Next Steps

### To Test Decoding:

#### Method 1: Decode One File
```bash
cd "d:\Work VS code\Rust-Projects\v-shield-fullstack"

# Decode test 1
./target/release/vshield-decode.exe \
  --input output/test_small \
  --output output/recovered_small.txt \
  --token "vshield://5650f5a0-d8c8-4d0c-a00c-46124b47d394"

# Verify it matches original
fc test_files\small.txt output\recovered_small.txt
```

#### Method 2: Decode All Files
```bash
# Create a batch script or run manually for each test
for token in "vshield://5650f5a0-d8c8-4d0c-a00c-46124b47d394" \
             "vshield://9c83bf99-f811-459f-aee1-3cfddae52208" \
             "vshield://00035e17-8302-46d1-a17a-12bb2bd00141"; do
  ./target/release/vshield-decode.exe --input output/test_$i --output output/recovered_$i.txt --token "$token"
done
```

---

## 📊 Expected Results

After decoding:
- ✅ All recovered files should be **byte-for-byte identical** to originals
- ✅ File hashes (SHA-256) should match
- ✅ Content should be readable and intact

### File Comparison Commands:
```bash
# Windows PowerShell
(Get-FileHash test_files\small.txt).Hash
(Get-FileHash output\recovered_small.txt).Hash
# Both hashes should be identical

# Windows CMD
fc test_files\small.txt output\recovered_small.txt
# Should say "no differences encountered"

# Linux/Git Bash
cmp test_files/small.txt output/recovered_small.txt
# Exit code 0 = files are identical
```

---

## 🔐 Important Notes

⚠️ **Tokens are crucial:**
- Each token is unique to its file
- Needed for decryption
- Cannot decode without the correct token
- Keep tokens safe!

📋 **Phase 1 Limitations:**
- File size limited by Reed-Solomon (255 symbol limit)
- Currently supports files up to ~100 bytes with 20% redundancy
- Decoder error correction is stubbed (returns data as-is for Phase 1.5)

🎬 **For YouTube:**
- Frames are PNG files that can be uploaded as video frames
- YouTube's compression may damage data (tested in Phase 1.5)
- Reed-Solomon ECC provides some protection

---

## 📁 Directory Structure

```
v-shield-fullstack/
├── test_files/
│   ├── small.txt              (50 bytes) ← Encoded ✅
│   ├── config_small.json      (22 bytes) ← Encoded ✅
│   ├── test_data.txt          (25 bytes) ← Encoded ✅
│   ├── document.txt           (Too large for Phase 1)
│   └── config.json            (Too large for Phase 1)
│
├── output/
│   ├── test_small/
│   │   ├── frame_0000.png     (Encoded data)
│   │   └── metadata.json
│   ├── test_config/
│   │   ├── frame_0000.png
│   │   └── metadata.json
│   ├── test_data/
│   │   ├── frame_0000.png
│   │   └── metadata.json
│   ├── recovered_small.txt    (Created after decoding)
│   ├── recovered_config.json  (Created after decoding)
│   └── recovered_test_data.txt (Created after decoding)
│
├── target/release/
│   ├── vshield-encode.exe     (Encoder binary)
│   └── vshield-decode.exe     (Decoder binary)
│
└── TOKENS.md                   (This file)
```

---

## ✨ Summary

✅ **Phase 1 Encoding: COMPLETE**
- 3 test files successfully encoded
- Tokens generated and saved
- PNG frames with embedded data created

⏭️ **Next: Phase 1 Decoding**
- Use tokens above to recover files
- Verify files match originals exactly
- Test error correction capabilities

📊 **Phase 1.5: YouTube Compatibility Testing**
- Compress frames with YouTube codec
- Verify data survives compression
- Measure resilience

---

## Questions?

See main documentation:
- `README.md` - Project overview
- `TESTING.md` - Full testing guide
- `PROJECT_STATUS.md` - Roadmap and progress
