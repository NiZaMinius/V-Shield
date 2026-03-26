# V-Shield Desktop Application (Tauri)

## Overview

The V-Shield Desktop Application is a Tauri-based application that provides a modern, user-friendly interface for encoding and decoding files. This document covers:

1. **Architecture & File Structure**
2. **Development Setup**
3. **Installation & Deployment**
4. **Building the UI**
5. **FFmpeg Integration**
6. **Platform-Specific Considerations**

---

## Architecture

### Technology Stack

| Layer | Technology | Purpose |
|-------|-----------|---------|
| **Backend** | Rust | Core encoding/decoding, crypto, performance |
| **Desktop Runtime** | Tauri | Cross-platform Native app, small size (~30MB) |
| **Frontend** | React/Svelte + TypeScript | Modern UI, responsive design |
| **IPC** | JSON-RPC | Communication between Rust and JS |
| **Bundling** | Webpack/Vite | Frontend build optimization |

### Directory Structure (Proposed)

```
apps/desktop/
├── src-tauri/           # Rust backend
│   ├── src/
│   │   ├── main.rs      # Tauri entry point
│   │   ├── commands/
│   │   │   ├── encode.rs
│   │   │   └── decode.rs
│   │   ├── ffmpeg/      # FFmpeg wrapper
│   │   └── lib.rs
│   ├── Cargo.toml
│   └── tauri.conf.json
├── src/                 # Frontend (React/Svelte)
│   ├── pages/
│   │   ├── Encoder.tsx
│   │   ├── Decoder.tsx
│   │   └── About.tsx
│   ├── components/
│   │   ├── FileUpload.tsx
│   │   ├── TokenDisplay.tsx
│   │   └── ProgressBar.tsx
│   ├── hooks/
│   │   └── useEncoder.ts
│   ├── App.tsx
│   └── index.tsx
├── public/
│   └── icon.png
├── tauri.conf.json
├── package.json
└── vite.config.ts
```

---

## Development Setup

### Prerequisites

```bash
# Install Node.js 16+ and Rust
node --version  # >= v16.0.0
cargo --version # >= 1.70.0

# Install Tauri CLI
npm install -g @tauri-apps/cli
```

### Project Initialization

```bash
# Create new Tauri project (if not already done)
npx create-tauri-app

# Or in existing project
npm install @tauri-apps/api @tauri-apps/cli -D

# Install frontend dependencies
npm install
```

### Configuration: `tauri.conf.json`

```json
{
  "build": {
    "beforeBuildCommand": "npm run build",
    "beforeDevCommand": "npm run dev",
    "devUrl": "http://localhost:3000",
    "frontendDist": "../dist"
  },
  "app": {
    "windows": [
      {
        "label": "main",
        "title": "V-Shield",
        "width": 1200,
        "height": 800,
        "resizable": true,
        "fullscreen": false
      }
    ],
    "security": {
      "csp": "default-src 'self'; img-src 'self' blob:"
    }
  },
  "tauri": {
    "allowlist": {
      "all": false,
      "fs": {
        "all": true,
        "scope": ["$DOCUMENT/*", "$DOWNLOAD/*"]
      },
      "dialog": {
        "open": true,
        "save": true
      },
      "http": {
        "all": false,
        "scope": ["https://**"]
      }
    },
    "security": {
      "csp": "default-src 'self'; script-src 'self' 'unsafe-eval'"
    }
  }
}
```

---

## Rust Backend Setup

### Cargo.toml (src-tauri)

```toml
[package]
name = "vshield-desktop"
version = "0.1.0"
edition = "2021"

[dependencies]
tauri = { version = "1.5", features = ["shell-open"] }
tokio = { version = "1", features = ["full"] }
vshield-core = { path = "../../crates/vshield-core" }
vshield-enc = { path = "../../crates/vshield-enc" }
vshield-dec = { path = "../../crates/vshield-dec" }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
sha2 = "0.10"
thiserror = "1.0"
log = "0.4"
env_logger = "0.10"
which = "4.4"  # For FFmpeg detection

[target.'cfg(windows)'.dependencies]
winapi = "0.3"
```

### Main Tauri Application: `src-tauri/src/main.rs`

```rust
#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

mod commands;
mod ffmpeg;
mod state;

use tauri::Manager;
use std::sync::Mutex;

#[derive(Default)]
struct AppState {
    current_encoding_progress: Mutex<f32>,
    current_decoding_progress: Mutex<f32>,
}

fn main() {
    env_logger::init();

    tauri::Builder::default()
        .manage(AppState::default())
        // Encoder commands
        .invoke_handler(tauri::generate_handler![
            commands::encode::encode_file,
            commands::encode::get_encoding_progress,
            commands::encode::cancel_encoding,
            
            // Decoder commands
            commands::decode::decode_file,
            commands::decode::get_decoding_progress,
            
            // Utility commands
            commands::utils::select_file,
            commands::utils::select_directory,
            commands::utils::check_ffmpeg,
            commands::utils::open_directory,
        ])
        .setup(|app| {
            // Initialize on startup
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
```

### Encoding Command: `src-tauri/src/commands/encode.rs`

```rust
use tauri::{State, AppHandle};
use vshield_enc::{Encoder, EncoderConfig};
use std::path::PathBuf;

#[tauri::command(rename_all = "snake_case")]
pub async fn encode_file(
    input_path: String,
    output_dir: String,
    block_size: u8,
    redundancy: u8,
    handle: AppHandle,
) -> Result<EncodeResult, String> {
    // Validate inputs
    if ![4, 8, 16].contains(&block_size) {
        return Err("Block size must be 4, 8, or 16".into());
    }

    // Create encoder config
    let config = EncoderConfig {
        input_file: input_path,
        output_file: output_dir,
        block_size,
        redundancy_percent: redundancy,
        frame_width: 1920,
        frame_height: 1080,
    };

    // Run encoding
    let encoder = Encoder::new(config);
    
    // Spawn in background thread to not block UI
    tokio::task::spawn_blocking(move || {
        match encoder.encode() {
            Ok(output) => {
                // Save frames
                if let Err(e) = output.save_as_images(&config.output_file) {
                    let _ = handle.emit_all("encode_error", e.to_string());
                    return;
                }

                let result = EncodeResult {
                    success: true,
                    token: output.token.clone(),
                    num_frames: output.num_frames,
                    metadata: output.metadata.clone(),
                };

                let _ = handle.emit_all("encode_complete", &result);
            }
            Err(e) => {
                let _ = handle.emit_all("encode_error", e.to_string());
            }
        }
    })
    .await
    .map_err(|e| format!("Encoding task failed: {}", e))
}

#[tauri::command]
pub async fn get_encoding_progress(
    state: State<'_, AppState>,
) -> Result<f32, String> {
    let progress = *state.current_encoding_progress.lock()
        .map_err(|e| format!("Lock error: {}", e))?;
    Ok(progress)
}

#[derive(serde::Serialize, Debug, Clone)]
pub struct EncodeResult {
    pub success: bool,
    pub token: String,
    pub num_frames: u32,
    pub metadata: serde_json::Value,
}
```

### Decoding Command: `src-tauri/src/commands/decode.rs`

```rust
use tauri::AppHandle;
use vshield_dec::{Decoder, DecoderConfig};

#[tauri::command(rename_all = "snake_case")]
pub async fn decode_file(
    input_dir: String,
    output_path: String,
    token: String,
    handle: AppHandle,
) -> Result<DecodeResult, String> {
    let config = DecoderConfig {
        input_frames_dir: input_dir,
        output_file: output_path.clone(),
        token,
    };

    tokio::task::spawn_blocking(move || {
        match Decoder::new(config).decode() {
            Ok(output) => {
                if let Err(e) = output.save(&output_path) {
                    let _ = handle.emit_all("decode_error", e.to_string());
                    return;
                }

                let result = DecodeResult {
                    success: true,
                    filename: output.metadata.filename,
                    file_size: output.metadata.file_size,
                };

                let _ = handle.emit_all("decode_complete", &result);
            }
            Err(e) => {
                let _ = handle.emit_all("decode_error", e.to_string());
            }
        }
    })
    .await
    .map_err(|e| format!("Decoding task failed: {}", e))
}

#[derive(serde::Serialize, Debug)]
pub struct DecodeResult {
    pub success: bool,
    pub filename: String,
    pub file_size: u64,
}
```

---

## FFmpeg Integration

### FFmpeg Module: `src-tauri/src/ffmpeg/mod.rs`

```rust
use std::process::{Command, Stdio};
use which::which;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum FFmpegError {
    #[error("FFmpeg not found. Please install FFmpeg.")]
    NotFound,
    #[error("FFmpeg execution failed: {0}")]
    ExecutionFailed(String),
    #[error("Invalid frame directory: {0}")]
    InvalidDirectory(String),
}

pub struct FFmpegManager;

impl FFmpegManager {
    /// Check if FFmpeg is installed
    pub fn check_installed() -> Result<String, FFmpegError> {
        which("ffmpeg")
            .map(|p| p.to_string_lossy().to_string())
            .map_err(|_| FFmpegError::NotFound)
    }

    /// Convert PNG frames to MP4
    pub fn frames_to_mp4(
        frames_dir: &str,
        output_path: &str,
        fps: u8,
        bitrate: &str,
    ) -> Result<(), FFmpegError> {
        let input_pattern = format!("{}\\frame_%04d.png", frames_dir);
        
        let status = Command::new("ffmpeg")
            .arg("-y") // Overwrite output
            .arg("-framerate").arg(fps.to_string())
            .arg("-i").arg(&input_pattern)
            .arg("-c:v").arg("libx264")
            .arg("-pix_fmt").arg("yuv420p")
            .arg("-b:v").arg(bitrate)
            .arg("-preset").arg("medium") // Balance speed/compression
            .arg(output_path)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .status()
            .map_err(|e| FFmpegError::ExecutionFailed(e.to_string()))?;

        if status.success() {
            Ok(())
        } else {
            Err(FFmpegError::ExecutionFailed("FFmpeg conversion failed".into()))
        }
    }

    /// Extract frames from video
    pub fn video_to_frames(
        video_path: &str,
        output_dir: &str,
    ) -> Result<u32, FFmpegError> {
        use std::fs;
        
        // Create output directory
        fs::create_dir_all(output_dir)
            .map_err(|e| FFmpegError::InvalidDirectory(e.to_string()))?;

        let output_pattern = format!("{}\\frame_%04d.png", output_dir);
        
        let status = Command::new("ffmpeg")
            .arg("-i").arg(video_path)
            .arg("-vf").arg("fps=30") // Extract at 30 FPS
            .arg(&output_pattern)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .status()
            .map_err(|e| FFmpegError::ExecutionFailed(e.to_string()))?;

        if status.success() {
            // Count frames
            let count = fs::read_dir(output_dir)
                .map_err(|e| FFmpegError::InvalidDirectory(e.to_string()))?
                .filter_map(|entry| {
                    entry.ok().and_then(|e| {
                        if e.path().extension().map(|ext| ext == "png").unwrap_or(false) {
                            Some(())
                        } else {
                            None
                        }
                    })
                })
                .count();
            
            Ok(count as u32)
        } else {
            Err(FFmpegError::ExecutionFailed("FFmpeg extraction failed".into()))
        }
    }
}
```

---

## Frontend (React Example)

### Main Component: `src/pages/Encoder.tsx`

```typescript
import React, { useState } from 'react';
import { invoke } from '@tauri-apps/api/tauri';
import { open as openDialog } from '@tauri-apps/api/dialog';
import './Encoder.css';

interface EncodeStatus {
  completed: boolean;
  token?: string;
  num_frames?: number;
  error?: string;
}

export const EncoderPage: React.FC = () => {
  const [inputFile, setInputFile] = useState<string>();
  const [outputDir, setOutputDir] = useState<string>();
  const [blockSize, setBlockSize] = useState(8);
  const [redundancy, setRedundancy] = useState(25);
  const [isEncoding, setIsEncoding] = useState(false);
  const [status, setStatus] = useState<EncodeStatus>({});
  const [token, setToken] = useState<string>();

  const handleSelectInput = async () => {
    const selected = await openDialog({
      filters: [{
        name: 'All Files',
        extensions: ['*'],
      }],
    });
    if (selected) setInputFile(selected as string);
  };

  const handleSelectOutput = async () => {
    const selected = await openDialog({
      directory: true,
    });
    if (selected) setOutputDir(selected as string);
  };

  const handleEncode = async () => {
    if (!inputFile || !outputDir) {
      alert('Please select input file and output directory');
      return;
    }

    setIsEncoding(true);
    try {
      const result: EncodeStatus = await invoke('encode_file', {
        inputPath: inputFile,
        outputDir: outputDir,
        blockSize,
        redundancy,
      });

      if (result.token) {
        setToken(result.token);
        setStatus(result);
      }
    } catch (error) {
      setStatus({ completed: true, error: String(error) });
    } finally {
      setIsEncoding(false);
    }
  };

  const handleCopyToken = () => {
    if (token) {
      navigator.clipboard.writeText(token);
      alert('Token copied to clipboard!');
    }
  };

  return (
   <div className="encoder-page">
      <h1>V-Shield Encoder</h1>
      
      <div className="form-group">
        <label>Input File</label>
        <div className="file-input">
          <input
            type="text"
            readOnly
            placeholder="Select a file"
            value={inputFile || ''}
          />
          <button onClick={handleSelectInput}>Browse</button>
        </div>
      </div>

      <div className="form-group">
        <label>Output Directory</label>
        <div className="file-input">
          <input
            type="text"
            readOnly
            placeholder="Select output directory"
            value={outputDir || ''}
          />
          <button onClick={handleSelectOutput}>Browse</button>
        </div>
      </div>

      <div className="settings-grid">
        <div>
          <label>Block Size (pixels)</label>
          <select value={blockSize} onChange={e => setBlockSize(Number(e.target.value))}>
            <option value={4}>4x4</option>
            <option value={8}>8x8</option>
            <option value={16}>16x16</option>
          </select>
        </div>

        <div>
          <label>Error Correction (%)</label>
          <select value={redundancy} onChange={e => setRedundancy(Number(e.target.value))}>
            <option value={20}>20%</option>
            <option value={25}>25%</option>
            <option value={30}>30%</option>
          </select>
        </div>
      </div>

      <button
        onClick={handleEncode}
        disabled={isEncoding || !inputFile || !outputDir}
        className="primary-button"
      >
        {isEncoding ? 'Encoding...' : 'Start Encoding'}
      </button>

      {token && (
        <div className="token-display">
          <h3>✓ Encoding Complete!</h3>
          <p>Frames saved: {status.num_frames}</p>
          <div className="token-box">
            <code>{token}</code>
            <button onClick={handleCopyToken}>Copy Token</button>
          </div>
          <p className="warning">⚠️ Save this token in a secure location!</p>
        </div>
      )}

      {status.error && (
        <div className="error-display">
          <p>Error: {status.error}</p>
        </div>
      )}
    </div>
  );
};
```

---

## Installation & Deployment

### Building for Distribution

```bash
# Development build
npm install
npm run tauri dev

# Production build
npm run tauri build

# Platform-specific:
# - Windows: target/release/v-shield.exe or MSI installer
# - macOS: target/release/app/V-Shield.app
# - Linux: AppImage or deb package
```

### Installation Methods

#### Windows

**Option 1: Direct EXE**
```cmd
v-shield-installer.exe
```

**Option 2: Windows Package Manager**
```cmd
winget install vshield
```

#### macOS

```bash
Installation via DMG or:
brew install v-shield
```

#### Linux  

```bash
# Ubuntu/Debian
sudo dpkg -i v-shield.deb

# Or Snap
sudo snap install v-shield
```

### Uninstallation

The application stores:
- **Config**: `~/.config/vshield/` (Linux/macOS) or `%APPDATA%\vshield\` (Windows)
- **Cache**: System cache directory
- **Tokens/History**: Local storage in app folder

**Clean Uninstall:**
```bash
# Windows: Use "Add/Remove Programs"

# macOS
rm -r ~/Library/Application\ Support/com.vshield.app
rm -r /Applications/V-Shield.app

# Linux
sudo apt remove vshield
rm -r ~/.config/vshield
```

---

## Security Considerations

### IPC Security

Tauri's IPC is secure by default, but:

```json
{
  "tauri": {
    "security": {
      "csp": "default-src 'self';",
      "dangerousUseHttpScheme": false
    }
  }
}
```

### File Access Scope

```json
{
  "fs": {
    "scope": [
      "$DOCUMENT/*",
      "$DOWNLOAD/*",
      "$DESKTOP/*"
    ]
  }
}
```

### Environment Variables

Never:
- ~~Embed FFmpeg paths hard-coded~~
- ~~Store tokens in config files~~
- ~~Log sensitive data~~

---

## Troubleshooting

### FFmpeg Not Found
```bash
# Install FFmpeg
#
# Windows (via Chocolatey)
choco install ffmpeg

# macOS
brew install ffmpeg

# Linux
sudo apt install ffmpeg
```

### Build Errors

```bash
# Clear cache
rm -rf src-tauri/target node_modules

# Rebuild
npm install && npm run tauri build
```

### IPC Communication Issues

Enable logging:
```rust
env_logger::Builder::from_default_env()
    .filter_level(log::LevelFilter::Debug)
    .init();
```

---

## Release Checklist

- [ ] All tests pass
- [ ] UI tested on Windows, macOS, Linux
- [ ] FFmpeg detection works
- [ ] Token display/copy works
- [ ] Encoding/decoding completes
- [ ] No console errors
- [ ] App installs cleanly
- [ ] App uninstalls cleanly
- [ ] Version bumped
- [ ] Release notes written

---

---

**Ready to build the desktop app? Start with Phase 2! 🚀**

