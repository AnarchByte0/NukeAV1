// Force build script rerun
use std::env;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::fs;

fn hack_portfile(vcpkg_dir: &Path) {
    let portfile = vcpkg_dir.join("ports").join("ffmpeg").join("portfile.cmake");
    
    // Always start with a clean portfile from git
    let _ = Command::new("git")
        .args(&["checkout", "--", "ports/ffmpeg/portfile.cmake"])
        .current_dir(vcpkg_dir)
        .status();

    let content = fs::read_to_string(&portfile).unwrap();
    
    let minimal_flags = "--disable-everything --enable-decoder=vp8,vp9,av1,libdav1d,libaom_av1,av1_cuvid,av1_qsv,aac,opus --enable-encoder=libvpx_vp8,libvpx_vp9,libaom_av1,av1_nvenc,hevc_nvenc,h264_nvenc,av1_amf,hevc_amf,h264_amf,av1_qsv,aac,opus --enable-parser=vp8,vp9,av1,aac,opus --enable-demuxer=matroska,webm,mov,mp4,m4a,3gp,3g2,mj2 --enable-muxer=webm,matroska,mp4,mov,ipod --enable-protocol=file";
    
    let new_content = content.replace(
        "set(OPTIONS \"--enable-pic",
        &format!("set(OPTIONS \"{} --enable-pic", minimal_flags)
    );
    fs::write(&portfile, new_content).unwrap();
}

fn run_command_streaming(cmd_program: &Path, cmd_args: &[&str], current_dir: &Path) -> std::process::ExitStatus {
    use std::io::{BufRead, BufReader};
    use std::process::{Command, Stdio};

    println!("cargo:warning=Running: {} {}", cmd_program.display(), cmd_args.join(" "));

    let mut child = Command::new(cmd_program)
        .args(cmd_args)
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .current_dir(current_dir)
        .spawn()
        .expect("Failed to spawn process");

    if let Some(stdout) = child.stdout.take() {
        let reader = BufReader::new(stdout);
        for line in reader.lines() {
            if let Ok(l) = line {
                println!("cargo:warning={}", l);
            }
        }
    }

    child.wait().expect("Failed to wait on child process")
}

fn main() {
    println!("cargo:rerun-if-changed=src/ffi/wrapper.hpp");
    println!("cargo:rerun-if-changed=sdk/Premiere Pro 26.0 C++ SDK Windows.zip");

    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    let manifest_dir = env::var("CARGO_MANIFEST_DIR").unwrap();
    
    // 1. Setup Adobe SDK
    let adobe_sdk_dir = out_dir.join("Adobe_Premiere_Pro_SDK");
    if !adobe_sdk_dir.exists() {
        let zip_path = PathBuf::from(&manifest_dir).join("sdk").join("Premiere Pro 26.0 C++ SDK Windows.zip");
        fs::create_dir_all(&adobe_sdk_dir).unwrap();
        
        let file = std::fs::File::open(&zip_path).expect("Failed to open Adobe SDK zip");
        let mut archive = zip::ZipArchive::new(file).expect("Failed to parse zip archive");
        archive.extract(&adobe_sdk_dir).expect("Failed to extract Adobe SDK");
    }
    
    // The zip crate preserves the top-level directory "Premiere Pro 26.0 C++ SDK"
    let sdk_headers = adobe_sdk_dir.join("Premiere Pro 26.0 C++ SDK").join("Examples").join("Headers");

    // 2. Setup VCPKG and Compile FFmpeg statically
    let vcpkg_dir = out_dir.join("vcpkg");
    if !vcpkg_dir.exists() {
        println!("cargo:warning=Cloning vcpkg and compiling FFmpeg from source. This will take 15-40 minutes!");
        
        let status = Command::new("git")
            .args(&["clone", "https://github.com/microsoft/vcpkg.git", vcpkg_dir.to_str().unwrap()])
            .status().unwrap();
        if !status.success() { panic!("Failed to clone vcpkg"); }

        let status = Command::new("cmd")
            .args(&["/C", "bootstrap-vcpkg.bat"])
            .current_dir(&vcpkg_dir)
            .status().unwrap();
        if !status.success() { panic!("Failed to bootstrap vcpkg"); }

        hack_portfile(&vcpkg_dir);

        let status = run_command_streaming(
            &vcpkg_dir.join("vcpkg.exe"),
            &[
                "install",
                "ffmpeg[core,avcodec,avformat,swscale,swresample,vpx,dav1d,aom,nvcodec,amf,qsv]:x64-windows-static",
                "--no-binarycaching"
            ],
            &vcpkg_dir
        );
            
        if !status.success() {
            panic!("Failed to install ffmpeg via vcpkg");
        }
    } else if env::var("SKIP_FFMPEG_UPDATE").is_err() {
        // --- AUTOMATIC FFMPEG UPDATE ---
        println!("cargo:warning=Checking for FFmpeg updates in vcpkg...");
        
        // 1. Revert our hacks to avoid git pull conflicts
        let _ = Command::new("git")
            .args(&["checkout", "--", "ports/ffmpeg/portfile.cmake"])
            .current_dir(&vcpkg_dir)
            .status();

        // 2. Fetch new versions from the repository
        let pull_status = Command::new("git")
            .args(&["pull"])
            .current_dir(&vcpkg_dir)
            .status().unwrap();

        if pull_status.success() {
            // 3. Re-apply our hacks
            hack_portfile(&vcpkg_dir);

            // 4. Update FFmpeg (if a new version is available)
            let _ = run_command_streaming(
                &vcpkg_dir.join("vcpkg.exe"),
                &[
                    "install",
                    "ffmpeg[core,avcodec,avformat,swscale,swresample,vpx,dav1d,aom,nvcodec,amf,qsv]:x64-windows-static",
                    "--no-binarycaching"
                ],
                &vcpkg_dir
            );
        }
    }

    unsafe {
        env::set_var("VCPKG_ROOT", &vcpkg_dir);
    }
    vcpkg::Config::new()
        .target_triplet("x64-windows-static")
        .find_package("ffmpeg")
        .expect("Failed to link ffmpeg from vcpkg");

    // Manually link required Windows system libraries for static FFmpeg
    println!("cargo:rustc-link-lib=Secur32");
    println!("cargo:rustc-link-lib=Ncrypt");
    println!("cargo:rustc-link-lib=Crypt32");
    println!("cargo:rustc-link-lib=Bcrypt");
    println!("cargo:rustc-link-lib=Ole32");
    println!("cargo:rustc-link-lib=User32");
    println!("cargo:rustc-link-lib=Advapi32");

    // 4. Generate Premiere Pro Bindings
    let bindings = bindgen::Builder::default()
        .header("src/ffi/wrapper.hpp")
        .clang_arg(format!("-I{}", sdk_headers.display()))
        .clang_arg("-x")
        .clang_arg("c++")
        .clang_arg("-std=c++14")
        .layout_tests(false)
        .parse_callbacks(Box::new(bindgen::CargoCallbacks::new()))
        .generate();

    match bindings {
        Ok(b) => {
            b.write_to_file(out_dir.join("pr_sdk_bindings.rs"))
                .expect("Couldn't write bindings!");
        }
        Err(e) => {
            panic!("Cannot generate bindings for Premiere Pro SDK: {:?}", e);
        }
    }
}
