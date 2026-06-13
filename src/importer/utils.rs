use std::ffi::CString;
use std::os::raw::c_char;
use std::ptr;
use crate::prUTF16Char;

pub fn copy_c_str(src: &str, dst: &mut [c_char]) {
    let c_str = CString::new(src).unwrap();
    let bytes = c_str.as_bytes_with_nul();
    let len = std::cmp::min(bytes.len(), dst.len());
    unsafe {
        ptr::copy_nonoverlapping(bytes.as_ptr() as *const c_char, dst.as_mut_ptr(), len);
        if len < dst.len() {
            dst[len] = 0;
        } else if dst.len() > 0 {
            dst[dst.len() - 1] = 0;
        }
    }
}

pub unsafe fn get_utf16_string(ptr: *const prUTF16Char) -> String {
    let mut len = 0;
    while *ptr.add(len) != 0 {
        len += 1;
    }
    let slice = std::slice::from_raw_parts(ptr, len);
    String::from_utf16_lossy(slice)
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
