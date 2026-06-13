use std::process::Command;
use std::path::PathBuf;
use std::fs;

fn main() {
    println!("=== NukeAV1 Plugin Builder ===");
    println!("Compiling NukeAV1 release...");

    // 1. Build the library (use release by default, but if this package tool is built in debug/dev, compile lib in debug/dev)
    let is_debug = cfg!(debug_assertions);
    let mut args = vec!["build", "--lib"];
    if !is_debug {
        args.push("--release");
    }

    let status = Command::new("cargo")
        .args(&args)
        .env("SKIP_FFMPEG_UPDATE", "1") // <- Забороняємо подвійну перевірку!
        .status()
        .expect("Failed to run cargo build");

    if !status.success() {
        eprintln!("Build failed!");
        std::process::exit(1);
    }

    // 2. Prepare paths
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap_or_else(|_| ".".into());
    let profile_dir = if is_debug { "debug" } else { "release" };
    let source_dll = PathBuf::from(&manifest_dir).join("target").join(profile_dir).join("NukeAV1.dll");
    
    // We will place the final .prm in an "output" folder inside the project
    let output_dir = PathBuf::from(&manifest_dir).join("output");
    let dest_prm = output_dir.join("NukeAV1.prm");

    if !output_dir.exists() {
        if let Err(e) = fs::create_dir_all(&output_dir) {
            eprintln!("Failed to create output directory: {}", e);
            std::process::exit(1);
        }
    }

    // 3. Copy the compiled DLL as a .prm file
    println!("Packaging NukeAV1.prm to {}...", output_dir.display());
    match fs::copy(&source_dll, &dest_prm) {
        Ok(_) => {
            println!("Success! Your plugin is ready at: {}", dest_prm.display());
            println!("To use it, copy this file to your Premiere Pro plugins folder!");
        }
        Err(e) => {
            eprintln!("Failed to package plugin: {}", e);
            std::process::exit(1);
        }
    }
}
