#![allow(dead_code)]
#![allow(unsafe_op_in_unsafe_fn)]
// NukeAV1 lib name is mandated by Adobe Premiere Pro plugin format (.prm/.dll must match)
#![allow(non_snake_case)]

pub mod ffi;
pub use crate::ffi::adobe::*;
pub use crate::ffi::ffmpeg as ffmpeg_ffi;

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

pub mod plugin;
pub mod utils;

pub use crate::plugin::importer;
pub use crate::plugin::exporter;
pub use crate::utils::{str_to_utf16, leak_utf16};

/// Main entry point for IMPORTER (AV1/VP9 reading)
#[unsafe(no_mangle)]
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
#[unsafe(no_mangle)]
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
