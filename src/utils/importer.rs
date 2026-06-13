pub fn should_avoid_audio_conform() -> bool {
    let appdata = match std::env::var("APPDATA") {
        Ok(val) => val,
        Err(_) => return false,
    };
    let dir = std::path::PathBuf::from(appdata).join("NukeAV1");
    if !dir.exists() {
        let _ = std::fs::create_dir_all(&dir);
    }
    let path = dir.join("config.txt");
    if !path.exists() {
        let _ = std::fs::write(
            &path,
            "# NukeAV1 Importer Configuration\n# Set to true to enable Premiere Pro audio cache files (.cfa/.pek)\nenable_audio_cache=true\n"
        );
    }
    if let Ok(content) = std::fs::read_to_string(&path) {
        if content.contains("enable_audio_cache=false") {
            return true;
        }
    }
    false
}
