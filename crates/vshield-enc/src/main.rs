use clap::Parser;
use std::path::PathBuf;
use vshield_enc::{Encoder, EncoderConfig};

#[derive(Parser, Debug)]
#[command(name = "V-Shield Encoder")]
#[command(about = "Convert files into YouTube-resistant video frames", long_about = None)]
struct Args {
    /// Input file to encode
    #[arg(short, long)]
    input: PathBuf,

    /// Output directory for frames
    #[arg(short, long)]
    output: PathBuf,

    /// Block size in pixels (4, 8, or 16)
    #[arg(short, long, default_value = "8")]
    block_size: u8,

    /// Error correction redundancy percentage (20-40)
    #[arg(short, long, default_value = "25")]
    redundancy: u8,

    /// Frame width in pixels (default 1920 for 1080p)
    #[arg(long, default_value = "1920")]
    width: u32,

    /// Frame height in pixels (default 1080 for 1080p)
    #[arg(long, default_value = "1080")]
    height: u32,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    println!("╔════════════════════════════════════╗");
    println!("║       V-Shield Encoder CLI         ║");
    println!("║     v0.1.0 - Phase 1 Alpha        ║");
    println!("╚════════════════════════════════════╝\n");

    // Validate inputs
    if !args.input.exists() {
        return Err(format!("Input file not found: {:?}", args.input).into());
    }

    if ![4, 8, 16].contains(&args.block_size) {
        return Err("Block size must be 4, 8, or 16".into());
    }

    if args.redundancy < 20 || args.redundancy > 40 {
        return Err("Redundancy must be between 20 and 40 percent".into());
    }

    let config = EncoderConfig {
        input_file: args.input.to_string_lossy().to_string(),
        output_file: args.output.to_string_lossy().to_string(),
        block_size: args.block_size,
        redundancy_percent: args.redundancy,
        frame_width: args.width,
        frame_height: args.height,
    };

    let encoder = Encoder::new(config);

    println!("📝 Configuration:");
    println!("   Input:      {:?}", args.input);
    println!("   Output:     {:?}", args.output);
    println!(
        "   Block Size: {}x{} pixels",
        args.block_size, args.block_size
    );
    println!("   Redundancy: {}%", args.redundancy);
    println!("   Resolution: {}x{}", args.width, args.height);
    println!();

    println!("🔧 Encoding...");
    let output = encoder.encode()?;

    println!("\n✅ Encoding complete!");
    println!("   Frames:     {}", output.num_frames);
    println!("   Token:      {}", output.token);
    println!("   File Hash:  {:x?}", &output.metadata.file_hash[..8]);
    println!();

    println!("💾 Saving output...");
    output.save_as_images(&args.output.to_string_lossy())?;

    println!("\n🎉 Success! Your video is ready.");
    println!("   ⚠️  Keep your token safe: {}", output.token);
    println!("   📤 Upload frames to YouTube as a video.");

    Ok(())
}
