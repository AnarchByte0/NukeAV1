# NukeAV1 🚀

A high-performance **AV1, VP8 & VP9 Importer and Exporter plugin for Adobe Premiere Pro**, built natively in Rust. It utilizes **FFmpeg** (via static build) to support decoding and encoding, with robust support for hardware acceleration (NVENC, AMF, QSV) and rich exporter settings.

[![Ko-Fi](https://img.shields.io/badge/Ko--fi-F16061?style=for-the-badge&logo=ko-fi&logoColor=white)](https://ko-fi.com/anarchbyte)
[![Buy Me A Coffee](https://img.shields.io/badge/Buy%20Me%20A%20Coffee-FFDD00?style=for-the-badge&logo=buy-me-a-coffee&logoColor=black)](https://buymeacoffee.com/anarchbyte)

---

## Key Features ✨

- **High-Performance Importer (Decoder):** Decode AV1 and VP9 streams natively within the Premiere Pro timeline.
- **Advanced Exporter (Encoder):** Encode sequences into AV1, VP8, and VP9 video streams.
- **Dynamic Adobe Preferences Synchronization:**
  - **Automatic Audio Cache Bypass:** Dynamically checks Premiere Pro's "Save media cache files next to originals" setting. If enabled, the plugin bypasses conforming `.cfa`/`.pek` files to prevent folder clutter; if disabled, it allows conformed cache in a centralized folder for better performance.
  - **Dynamic RAM Cache Allocation:** Uses Windows FFI to fetch system RAM and automatically scales the timeline scrubbing cache size (from 4 up to 32 frames) for smooth navigation.
  - **Global GPU Acceleration Sync:** Disables hardware-accelerated decoding fallbacks automatically if Premiere Pro's project renderer is set to Software Only.
  - **Sequence Auto-fill:** Auto-populates default export parameters (width, height, FPS) matching your most recent sequence settings in Premiere Pro.
- **Multiplexer Control:** Dynamic multiplexer options (`MP4`, `3GPP`, `None`), with automated audio stream silencing when `None` is selected.

---

## Architecture Overview 🏗️

The project is structured as follows:
- **`src/lib.rs`**: Main entry points (`xImportEntry` and `xSDKExport`) registered for Premiere Pro SDK FFI.
- **`src/ffi/`**: Isolated direct FFI bindings.
  - `adobe.rs`: Auto-generated bindings to Premiere Pro C++ SDK.
  - `ffmpeg.rs`: FFI bindings to static FFmpeg libraries.
- **`src/plugin/`**: Core video plugin logic.
  - `importer/`: Handles importing, frame-by-frame decoding, and asynchronous parsing of AV1/VP9 video.
  - `exporter/`: Manages exporting configurations, multiplexer preferences, and interfacing with FFmpeg encoders.
  - `shared.rs`: Common pixel format and color space conversion utilities shared between importer and exporter to minimize binary size.
- **`src/utils/`**: Shared helper files (UIBuilder, cache preference check, string conversions).

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

## Support the Project ☕

If you find this plugin useful and want to support its active development, you can sponsor or donate through these platforms:

[![Ko-Fi](https://img.shields.io/badge/Ko--fi-F16061?style=for-the-badge&logo=ko-fi&logoColor=white)](https://ko-fi.com/anarchbyte)
[![Buy Me A Coffee](https://img.shields.io/badge/Buy%20Me%20A%20Coffee-FFDD00?style=for-the-badge&logo=buy-me-a-coffee&logoColor=black)](https://buymeacoffee.com/anarchbyte)

---

## License 📄

This project is licensed under the **GNU General Public License v3.0** with the **Adobe SDK Linking Exception** under the name **AnarchByte**. See the [LICENSE](LICENSE) file for details.

FFmpeg is licensed under the LGPL/GPL; compilation artifacts must respect individual third-party licenses when packaged.
