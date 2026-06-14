use std::ffi::c_void;
use std::os::raw::c_int;

use crate::*;
use crate::exporter::startup::handle_startup;
use crate::exporter::export::handle_export;
use crate::exporter::params::handle_generate_default_params;

pub mod startup;
pub mod params;
pub mod query;
pub mod export;
pub mod ffmpeg;

/// Selector handler for Exporter
pub unsafe fn handle_export_selector(
    selector: c_int,
    std_parms: *mut exportStdParms,
    param1: *mut c_void,
    _param2: *mut c_void,
) -> prMALError {
    #[allow(non_upper_case_globals)]
    match selector as PrExportSelector {
        PrExportSelector_exSelStartup => handle_startup(param1),
        PrExportSelector_exSelBeginInstance => malNoError as prMALError,
        PrExportSelector_exSelExport => handle_export(std_parms, param1),
        PrExportSelector_exSelEndInstance => malNoError as prMALError,
        PrExportSelector_exSelShutdown => malNoError as prMALError,
        PrExportSelector_exSelGenerateDefaultParams => handle_generate_default_params(std_parms, param1),
        PrExportSelector_exSelQueryOutputSettings => crate::exporter::query::handle_query_output_settings(std_parms, param1),
        PrExportSelector_exSelGetParamSummary => crate::exporter::query::handle_query_param_summary(std_parms, param1),
        PrExportSelector_exSelPostProcessParams => crate::exporter::params::handle_post_process_params(std_parms, param1),
        PrExportSelector_exSelValidateOutputSettings => malNoError as prMALError,
        PrExportSelector_exSelValidateParamChanged => crate::exporter::params::handle_validate_param_changed(std_parms, param1),
        PrExportSelector_exSelQueryExportFileExtension => crate::exporter::query::handle_query_export_file_extension(std_parms, param1),
        PrExportSelector_exSelQueryExportColorSpace => crate::exporter::query::handle_query_export_color_space(std_parms, param1),
        _ => PrExportReturnValue_exportReturn_Unsupported as prMALError,
    }
}
