#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(dead_code)]
#![allow(unsafe_op_in_unsafe_fn)]

// Include generated structures from bindgen
include!(concat!(env!("OUT_DIR"), "/pr_sdk_bindings.rs"));

use std::ffi::c_void;
use std::os::raw::c_int;

#[macro_export]
macro_rules! log_debug {
    ($($arg:tt)*) => {
        if let Ok(mut file) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(std::env::temp_dir().join("NukeAV1_debug.log"))
        {
            use std::io::Write;
            let _ = writeln!(file, $($arg)*);
        }
    };
}

pub mod exporter;
pub mod importer;
pub mod utils;
pub mod ffmpeg_ffi;

/// Main entry point for IMPORTER (AV1/VP9 reading)
#[no_mangle]
pub unsafe extern "C" fn xImportEntry(
    selector: c_int,
    std_parms: *mut imStdParms,
    param1: *mut c_void,
    param2: *mut c_void,
) -> prMALError {
    log_debug!("xImportEntry called: selector = {}, param1 = {:?}, param2 = {:?}", selector, param1, param2);

    if std_parms.is_null() {
        return malUnknownError as prMALError;
    }
    
    let res = importer::handle_import_selector(selector, std_parms, param1, param2);
    log_debug!("xImportEntry returned: {} for selector = {}", res, selector);
    res
}

/// Entry point for exporter module (AV1/VP8/VP9 encoding)
#[no_mangle]
pub unsafe extern "C" fn xSDKExport(
    selector: c_int,
    std_parms: *mut exportStdParms,
    param1: *mut c_void,
    param2: *mut c_void,
) -> prMALError {

    if std_parms.is_null() {
        return malUnknownError as prMALError;
    }

    exporter::handle_export_selector(selector, std_parms, param1, param2)
}

/// Utility to copy Rust string into Adobe UTF-16 buffer
pub unsafe fn str_to_utf16(s: &str, out: *mut prUTF16Char, max_len: usize) {
    let mut i = 0;
    for c in s.encode_utf16() {
        if i >= max_len - 1 {
            break;
        }
        *out.add(i) = c;
        i += 1;
    }
    *out.add(i) = 0; // null-terminator
}

/// Utility to create a persistent UTF-16 string (for SDK)
pub fn leak_utf16(s: &str) -> *const prUTF16Char {
    let mut vec: Vec<prUTF16Char> = s.encode_utf16().collect();
    vec.push(0); // null-terminator
    Box::leak(vec.into_boxed_slice()).as_ptr()
}
