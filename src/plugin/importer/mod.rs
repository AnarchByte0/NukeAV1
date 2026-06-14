use std::ffi::c_void;
use std::os::raw::c_int;

use crate::*;
use crate::importer::format::{handle_get_ind_format, handle_get_ind_pixel_format};
use crate::importer::info::{handle_get_info8, handle_get_info9};
use crate::importer::file::{handle_open_file8, handle_quiet_file, handle_close_file};
use crate::importer::image::handle_import_image;
use crate::importer::audio::handle_import_audio;

pub mod types;
pub mod format;
pub mod info;
pub mod file;
pub mod image;
pub mod audio;
pub mod async_importer;

pub unsafe fn handle_import_selector(
    selector: c_int,
    _std_parms: *mut imStdParms,
    param1: *mut c_void,
    param2: *mut c_void,
) -> prMALError {
    #[allow(non_upper_case_globals)]
    match selector as PrImporterSelector {
        PrImporterSelector_imInit => {
            let import_info = param1 as *mut imImportInfoRec;
            if !import_info.is_null() {
                (*import_info).canSave = 0;
                (*import_info).canDelete = 0;
                (*import_info).canResize = 1;
                (*import_info).hasSetup = 0;
                (*import_info).setupOnDblClk = 0;
                (*import_info).priority = 100;
                (*import_info).canProvidePeakAudio = 0;
                (*import_info).canAsync = 1;
                (*import_info).keepLoaded = 1;
                if crate::utils::importer::should_avoid_audio_conform() {
                    (*import_info).avoidAudioConform = 1;
                } else {
                    (*import_info).avoidAudioConform = 0;
                }
            }
            malNoError as prMALError
        }
        PrImporterSelector_imShutdown => malNoError as prMALError,
        PrImporterSelector_imGetIndFormat => handle_get_ind_format(param1, param2),
        PrImporterSelector_imGetIndPixelFormat => handle_get_ind_pixel_format(param1, param2),
        PrImporterSelector_imGetInfo8 => handle_get_info8(_std_parms, param1, param2),
        PrImporterSelector_imGetInfo9 => handle_get_info9(_std_parms, param1, param2),
        PrImporterSelector_imOpenFile8 => handle_open_file8(_std_parms, param1, param2),
        PrImporterSelector_imQuietFile => handle_quiet_file(param1),
        PrImporterSelector_imCloseFile => handle_close_file(param2),
        PrImporterSelector_imImportImage => handle_import_image(param1, param2),
        PrImporterSelector_imImportAudio7 => handle_import_audio(param1, param2),
        PrImporterSelector_imGetPeakAudio => crate::importer::audio::handle_get_peak_audio(param1, param2),
        PrImporterSelector_imCreateAsyncImporter => crate::importer::async_importer::handle_create_async_importer(_std_parms, param1, param2),
        PrImporterSelector_imGetSourceVideo => crate::importer::async_importer::handle_get_source_video(param1, param2),
        PrImporterSelector_imGetSupports7 => malSupports7 as prMALError,
        PrImporterSelector_imGetSupports8 => malSupports8 as prMALError,
        PrImporterSelector_imGetSupportsPerInstancePrefs => malSupportsPerInstancePrefs as prMALError,
        _ => PrImporterReturnValue_imUnsupported as prMALError,
    }
}
