use std::ffi::c_void;
use crate::*;

pub unsafe fn handle_startup(param1: *mut c_void) -> prMALError {
    let info = param1 as *mut exExporterInfoRec;
    if !info.is_null() {
        let info = &mut *info;
        
        let class_id = u32::from_be_bytes(*b"NukE");
        
        if info.exportReqIndex == 0 {
            info.fileType = u32::from_be_bytes(*b"NukA");
            info.classID = class_id;
            crate::str_to_utf16("AV1 (Nuke)", core::ptr::addr_of_mut!(info.fileTypeName) as *mut prUTF16Char, 256);
            crate::str_to_utf16("mp4", core::ptr::addr_of_mut!(info.fileTypeDefaultExtension) as *mut prUTF16Char, 256);
            
            // Instead of info.exportReqIndex = ..., we must return IterateExporter at the end
        } else if info.exportReqIndex == 1 {
            info.fileType = u32::from_be_bytes(*b"Nuk8");
            info.classID = class_id;
            crate::str_to_utf16("VP8 (Nuke)", core::ptr::addr_of_mut!(info.fileTypeName) as *mut prUTF16Char, 256);
            crate::str_to_utf16("webm", core::ptr::addr_of_mut!(info.fileTypeDefaultExtension) as *mut prUTF16Char, 256);
            
        } else if info.exportReqIndex == 2 {
            info.fileType = u32::from_be_bytes(*b"Nuk9");
            info.classID = class_id;
            crate::str_to_utf16("VP9 (Nuke)", core::ptr::addr_of_mut!(info.fileTypeName) as *mut prUTF16Char, 256);
            crate::str_to_utf16("webm", core::ptr::addr_of_mut!(info.fileTypeDefaultExtension) as *mut prUTF16Char, 256);
            
        } else {
            return crate::PrExportReturnValue_exportReturn_IterateExporterDone as prMALError;
        }

        info.wantsNoProgressBar = 0;
        info.hideInUI = 0;
        info.doesNotSupportAudioOnly = 0;
        info.canExportVideo = 1;
        info.canExportAudio = 1;
        info.singleFrameOnly = 0;
        info.interfaceVersion = EXPORTMOD_VERSION as i32;
        info.isCacheable = 1;
        info.canConformToMatchParams = 1;
        
        if info.exportReqIndex < 2 {
            return crate::PrExportReturnValue_exportReturn_IterateExporter as prMALError;
        } else {
            return malNoError as prMALError;
        }
    }
    malNoError as prMALError
}
