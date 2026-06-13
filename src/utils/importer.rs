fn get_adobe_files_side_by_side() -> Option<bool> {
    let user_profile = std::env::var("USERPROFILE").ok()?;
    let path = std::path::PathBuf::from(user_profile)
        .join("Documents")
        .join("Adobe")
        .join("Premiere Pro");
    
    if !path.exists() {
        return None;
    }
    
    // Find all version subdirectories
    let mut versions = Vec::new();
    if let Ok(entries) = std::fs::read_dir(&path) {
        for entry in entries {
            if let Ok(entry) = entry {
                let file_name = entry.file_name();
                let name_str = file_name.to_string_lossy();
                if entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
                    if name_str.chars().all(|c| c.is_digit(10) || c == '.') {
                        versions.push(name_str.into_owned());
                    }
                }
            }
        }
    }
    
    // Sort versions to get the highest (latest version of Premiere Pro)
    versions.sort_by(|a, b| {
        let parse = |s: &str| -> Vec<u32> {
            s.split('.').filter_map(|x| x.parse::<u32>().ok()).collect()
        };
        parse(b).cmp(&parse(a))
    });
    
    for version in versions {
        let version_path = path.join(version);
        // Find Profile directory (e.g. Profile-username)
        if let Ok(entries) = std::fs::read_dir(&version_path) {
            for entry in entries {
                if let Ok(entry) = entry {
                    let file_name = entry.file_name();
                    let name_str = file_name.to_string_lossy();
                    if name_str.starts_with("Profile-") {
                        let prefs_file = version_path.join(name_str.as_ref()).join("Adobe Premiere Pro Prefs");
                        if prefs_file.exists() {
                            if let Ok(content) = std::fs::read_to_string(&prefs_file) {
                                if content.contains("<BE.Prefs.MediaCache.FilesSideBySide>true</BE.Prefs.MediaCache.FilesSideBySide>") {
                                    return Some(true);
                                }
                                if content.contains("<BE.Prefs.MediaCache.FilesSideBySide>false</BE.Prefs.MediaCache.FilesSideBySide>") {
                                    return Some(false);
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    None
}

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
            "# NukeAV1 Importer Configuration\n\
             # Set to true/false to override, or auto to follow Adobe preferences:\n\
             # - If 'Save Media Cache files next to originals' is enabled in Adobe, we disable audio cache to avoid folder clutter.\n\
             # - If disabled in Adobe, we enable audio cache for better performance since it goes to a centralized directory.\n\
             enable_audio_cache=auto\n"
        );
    }
    
    if let Ok(content) = std::fs::read_to_string(&path) {
        if content.contains("enable_audio_cache=true") {
            return false; // Force enable audio cache (do NOT avoid conforming)
        }
        if content.contains("enable_audio_cache=false") {
            return true; // Force disable audio cache (avoid conforming)
        }
    }
    
    // In "auto" mode:
    if let Some(side_by_side) = get_adobe_files_side_by_side() {
        return side_by_side; // if side-by-side (next to originals) is true, we avoid conforming.
    }
    
    true // default fallback
}
