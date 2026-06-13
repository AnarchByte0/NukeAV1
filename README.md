# NukeAV1 🚀

A high-performance **AV1 & VP9 Importer and Exporter plugin for Adobe Premiere Pro**, built natively in Rust. It utilizes **FFmpeg** (via static build) to support decoding and encoding, with robust support for hardware acceleration (NVENC, AMF, QSV) and rich exporter settings.

---

## Key Features ✨

- **High-Performance Importer (Decoder):** Decode AV1 and VP9 streams natively within the Premiere Pro timeline.
- **Advanced Exporter (Encoder):** Encode sequences into AV1, VP8, and VP9 video streams.
- **Hardware/GPU Acceleration (VRAM):** Harness hardware capabilities (`nvcodec`, `amf`, `qsv`) to shift resource processing directly from RAM to VRAM.
- **Multiplexer Control:** Dynamic multiplexer options (`MP4`, `3GPP`, `None`), with automated audio stream silencing when `None` is selected.
- **Audio Conforming Bypass:** Optimized to reduce CPU rendering overhead for audio configurations.

---

## Architecture Overview 🏗️

The project is structured as follows:
- **`src/lib.rs`**: Main entry points (`xImportEntry` and `xSDKExport`) registered for Premiere Pro SDK FFI.
- **`src/importer/`**: Handles importing, frame-by-frame decoding, pixel format conversions, and asynchronous parsing of AV1/VP9 video.
- **`src/exporter/`**: Manages exporting configurations, multiplexer preferences, and interfacing with FFmpeg encoders.
- **`src/ffmpeg_ffi.rs`**: Direct FFI bindings to static FFmpeg libraries.

---

## Build Instructions 🛠️

### Prerequisites
1. **Windows OS**
2. **Rust (Edition 2024)** installed via Rustup.
3. **Visual Studio Build Tools** (MSVC C++ compiler).
4. **Git** and **CMake** (required for `vcpkg` to build FFmpeg).

### Setting up the Adobe Premiere Pro C++ SDK
Since the Adobe SDK contains proprietary headers, it is not distributed directly in this repository:
1. Download the **Premiere Pro C++ SDK** (Version 26.0 recommended) from the Adobe Developer Console.
2. Put the zip file renamed as `Premiere Pro 26.0 C++ SDK Windows.zip` into the `sdk/` directory, OR extract it manually so that the `Examples/Headers` directory is reachable inside the output directory during compilation.

### Building
Build the release binary by running:
```bash
cargo build --release
```
> [!NOTE]
> **First Compile Duration:** The first run triggers `vcpkg` to download and statically build FFmpeg with specific decoder/encoder flags. This process is fully automated but takes **15 to 40 minutes** depending on your CPU. Subsequent compilations take just a few seconds.

---

## Installation 📦

Once compiled, locate the generated library file:
- Target path: `target/release/NukeAV1.dll` (or compiled package equivalent)

Rename or copy the output binary file to:
```
C:\Program Files\Adobe\Common\Plug-ins\7.0\MediaCore\NukeAV1.prm
```
Restart Adobe Premiere Pro to load the new plugin.

---

## License 📄

This project is licensed under the **MIT License**. See the [LICENSE](LICENSE) file for details.
FFmpeg is licensed under the LGPL/GPL; compilation artifacts must respect individual third-party licenses when packaged.
