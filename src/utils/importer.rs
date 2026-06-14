use std::path::PathBuf;

#[cfg(target_os = "windows")]
#[repr(C)]
#[allow(non_snake_case)]
struct MEMORYSTATUSEX {
    dwLength: u32,
    dwMemoryLoad: u32,
    ullTotalPhys: u64,
    ullAvailPhys: u64,
    ullTotalPageFile: u64,
    ullAvailPageFile: u64,
    ullTotalVirtual: u64,
    ullAvailVirtual: u64,
    ullAvailExtendedVirtual: u64,
}

#[cfg(target_os = "windows")]
unsafe extern "system" {
    fn GlobalMemoryStatusEx(lpBuffer: *mut MEMORYSTATUSEX) -> i32;
}

pub fn get_adobe_pref_str(key: &str) -> Option<String> {
    let base_dir = if cfg!(target_os = "windows") {
        std::env::var("USERPROFILE").ok()
    } else {
        std::env::var("HOME").ok()
    }?;
    
    let path = PathBuf::from(base_dir)
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
                                let start_tag = format!("<{}>", key);
                                let end_tag = format!("</{}>", key);
                                if let Some(start_idx) = content.find(&start_tag) {
                                    if let Some(end_idx) = content.find(&end_tag) {
                                        let val_start = start_idx + start_tag.len();
                                        if end_idx > val_start {
                                            return Some(content[val_start..end_idx].to_string());
                                        }
                                    }
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

pub fn get_adobe_pref_bool(key: &str) -> Option<bool> {
    get_adobe_pref_str(key).map(|s| s.trim() == "true")
}

pub fn is_gpu_acceleration_enabled() -> bool {
    get_adobe_pref_bool("MZ.Prefs.UseGpuAcceleration").unwrap_or(true)
}

pub fn get_dynamic_cache_size() -> usize {
    let mut total_gb = 16; // default fallback

    #[cfg(target_os = "windows")]
    {
        let mut mem_info = MEMORYSTATUSEX {
            dwLength: std::mem::size_of::<MEMORYSTATUSEX>() as u32,
            dwMemoryLoad: 0,
            ullTotalPhys: 0,
            ullAvailPhys: 0,
            ullTotalPageFile: 0,
            ullAvailPageFile: 0,
            ullTotalVirtual: 0,
            ullAvailVirtual: 0,
            ullAvailExtendedVirtual: 0,
        };
        if unsafe { GlobalMemoryStatusEx(&mut mem_info) } != 0 {
            total_gb = mem_info.ullTotalPhys / (1024 * 1024 * 1024);
        }
    }

    #[cfg(target_os = "macos")]
    {
        let mut mem: u64 = 0;
        let mut len = std::mem::size_of::<u64>();
        unsafe {
            if let Ok(name) = std::ffi::CString::new("hw.memsize") {
                if libc::sysctlbyname(name.as_ptr(), &mut mem as *mut u64 as *mut _, &mut len, std::ptr::null_mut(), 0) == 0 {
                    total_gb = mem / (1024 * 1024 * 1024);
                }
            }
        }
    }

    if total_gb < 8 {
        4
    } else if total_gb < 16 {
        8
    } else if total_gb < 32 {
        16
    } else {
        32 // 32 GB or more: cache up to 32 frames for smooth scrubbing
    }
}

pub fn should_avoid_audio_conform() -> bool {
    let appdata = if cfg!(target_os = "windows") {
        std::env::var("APPDATA").ok()
    } else {
        std::env::var("HOME").ok().map(|h| format!("{}/Library/Application Support", h))
    };
    
    let appdata_val = match appdata {
        Some(val) => val,
        None => return false,
    };
    
    let dir = PathBuf::from(appdata_val).join("NukeAV1");
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
    if let Some(side_by_side) = get_adobe_pref_bool("BE.Prefs.MediaCache.FilesSideBySide") {
        return side_by_side; // if side-by-side (next to originals) is true, we avoid conforming.
    }
    
    true // default fallback
}
