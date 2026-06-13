use std::ffi::c_void;
use crate::*;
use crate::importer::utils::copy_c_str;

pub unsafe fn handle_get_ind_format(param1: *mut c_void, param2: *mut c_void) -> prMALError {
    let index = param1 as usize;
    let format_rec = param2 as *mut imIndFormatRec;

    if format_rec.is_null() {
        return malUnknownError as prMALError;
    }

    let format_rec = &mut *format_rec;

    if index == 0 {
        format_rec.filetype = i32::from_be_bytes(*b"MKV ");
        copy_c_str("Matroska Video (Nuke)", &mut format_rec.FormatName);
        copy_c_str("mkv", &mut format_rec.FormatShortName);
        copy_c_str("mkv", &mut format_rec.PlatformExtension);
        format_rec.flags = 0;
        malNoError as prMALError
    } else if index == 1 {
        format_rec.filetype = i32::from_be_bytes(*b"WEBM");
        copy_c_str("WebM Video (Nuke)", &mut format_rec.FormatName);
        copy_c_str("webm", &mut format_rec.FormatShortName);
        copy_c_str("webm", &mut format_rec.PlatformExtension);
        format_rec.flags = 0;
        malNoError as prMALError
    } else if index == 2 {
        format_rec.filetype = i32::from_be_bytes(*b"MP4 ");
        copy_c_str("MPEG-4 Video (Nuke)", &mut format_rec.FormatName);
        copy_c_str("mp4", &mut format_rec.FormatShortName);
        copy_c_str("mp4", &mut format_rec.PlatformExtension);
        format_rec.flags = 0;
        malNoError as prMALError
    } else if index == 3 {
        format_rec.filetype = i32::from_be_bytes(*b"MOV ");
        copy_c_str("QuickTime Movie (Nuke)", &mut format_rec.FormatName);
        copy_c_str("mov", &mut format_rec.FormatShortName);
        copy_c_str("mov", &mut format_rec.PlatformExtension);
        format_rec.flags = 0;
        malNoError as prMALError
    } else {
        imBadFormatIndex as prMALError
    }
}

pub unsafe fn handle_get_ind_pixel_format(param1: *mut c_void, param2: *mut c_void) -> prMALError {
    let index = param1 as usize;
    let rec = param2 as *mut imIndPixelFormatRec;
    if rec.is_null() {
        return malUnknownError as prMALError;
    }
    
    let rec = &mut *rec;
    
    match index {
        0 => {
            rec.outPixelFormat = PrPixelFormat_PrPixelFormat_BGRA_4444_8u;
            malNoError as prMALError
        }
        1 => {
            rec.outPixelFormat = PrPixelFormat_PrPixelFormat_BGRA_4444_16u;
            malNoError as prMALError
        }
        2 => {
            rec.outPixelFormat = PrPixelFormat_PrPixelFormat_BGRA_4444_32f;
            malNoError as prMALError
        }
        _ => imBadFormatIndex as prMALError,
    }
}
