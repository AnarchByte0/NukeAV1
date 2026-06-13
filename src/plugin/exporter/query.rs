use std::ffi::c_void;
use std::os::raw::c_char;
use crate::*;

pub unsafe fn handle_query_output_settings(std_parms: *mut exportStdParms, param1: *mut c_void) -> prMALError {
    let qs = param1 as *mut exQueryOutputSettingsRec;
    if !qs.is_null() && !std_parms.is_null() {
        let qs = &mut *qs;
        let std_parms = &*std_parms;
        
        if let Some(get_basic_suite) = std_parms.getSPBasicSuite {
            let basic_suite_ptr = get_basic_suite();
            if !basic_suite_ptr.is_null() {
                let basic_suite = &*(basic_suite_ptr as *const SPBasicSuite);
                if let Some(acquire_suite) = basic_suite.AcquireSuite {
                    let mut param_suite_ptr: *const c_void = core::ptr::null();
                    let err = acquire_suite(
                        kPrSDKExportParamSuite.as_ptr() as *const c_char,
                        kPrSDKExportParamSuiteVersion as i32,
                        &mut param_suite_ptr
                    );
                    
                    if err == 0 && !param_suite_ptr.is_null() {
                        let param_suite = &*(param_suite_ptr as *const PrSDKExportParamSuite);
                        
                        if let Some(get_param_value) = param_suite.GetParamValue {
                            let mut val: exParamValues = core::mem::zeroed();
                            
                            if qs.inExportVideo != 0 {
                                get_param_value(qs.exporterPluginID, 0, ADBEVideoWidth.as_ptr() as *const c_char, &mut val);
                                qs.outVideoWidth = val.value.__bindgen_anon_1.intValue;
                                
                                get_param_value(qs.exporterPluginID, 0, ADBEVideoHeight.as_ptr() as *const c_char, &mut val);
                                qs.outVideoHeight = val.value.__bindgen_anon_1.intValue;
                                
                                get_param_value(qs.exporterPluginID, 0, ADBEVideoFPS.as_ptr() as *const c_char, &mut val);
                                qs.outVideoFrameRate = val.value.__bindgen_anon_1.timeValue;
                                
                                get_param_value(qs.exporterPluginID, 0, ADBEVideoAspect.as_ptr() as *const c_char, &mut val);
                                qs.outVideoAspectNum = val.value.__bindgen_anon_1.ratioValue.numerator;
                                qs.outVideoAspectDen = val.value.__bindgen_anon_1.ratioValue.denominator;
                                
                                get_param_value(qs.exporterPluginID, 0, ADBEVideoFieldType.as_ptr() as *const c_char, &mut val);
                                qs.outVideoFieldType = val.value.__bindgen_anon_1.intValue;
                            }
                            
                            let mut target_bitrate = 10.0;
                            let mut audio_bitrate_choice = 1;

                            if qs.inExportAudio != 0 {
                                get_param_value(qs.exporterPluginID, 0, ADBEAudioRatePerSecond.as_ptr() as *const c_char, &mut val);
                                qs.outAudioSampleRate = val.value.__bindgen_anon_1.floatValue;
                                qs.outAudioSampleType = PrAudioSampleType_kPrAudioSampleType_32BitFloat;
                                
                                get_param_value(qs.exporterPluginID, 0, ADBEAudioNumChannels.as_ptr() as *const c_char, &mut val);
                                qs.outAudioChannelType = val.value.__bindgen_anon_1.intValue;
                            }

                            if qs.inExportVideo != 0 {
                                if get_param_value(qs.exporterPluginID, qs.inMultiGroupIndex, ADBEVideoTargetBitrate.as_ptr() as *const c_char, &mut val) == 0 {
                                    target_bitrate = val.value.__bindgen_anon_1.floatValue;
                                }
                            }
                            if qs.inExportAudio != 0 {
                                if get_param_value(qs.exporterPluginID, qs.inMultiGroupIndex, ADBEAudioBitrate.as_ptr() as *const c_char, &mut val) == 0 {
                                    audio_bitrate_choice = val.value.__bindgen_anon_1.intValue;
                                }
                            }

                            let mut total_bitrate_bps = 0.0;
                            if qs.inExportVideo != 0 {
                                total_bitrate_bps += target_bitrate * 1_000_000.0;
                            }
                            if qs.inExportAudio != 0 {
                                let audio_bps = match audio_bitrate_choice {
                                    0 => 96000.0,
                                    1 => 128000.0,
                                    2 => 160000.0,
                                    3 => 192000.0,
                                    4 => 256000.0,
                                    5 => 320000.0,
                                    _ => 128000.0,
                                };
                                total_bitrate_bps += audio_bps;
                            }
                            // According to SDK: "return outBitratePerSecond in kbps (outBitratePerSecond = bitrate_bps / 1000)" or as SDK code shows: outputSettingsP->outBitratePerSecond = outputSettingsP->outBitratePerSecond * 8 / 1000 if it was calculated using bytes.
                            // If total_bitrate_bps is in bits per second, we just divide by 1000.0 to get kbps.
                            qs.outBitratePerSecond = (total_bitrate_bps / 1000.0) as u32;
                        }
                        
                        if let Some(release_suite) = basic_suite.ReleaseSuite {
                            release_suite(kPrSDKExportParamSuite.as_ptr() as *const i8, kPrSDKExportParamSuiteVersion as i32);
                        }
                    }
                }
            }
        }
        
        qs.outUseMaximumRenderPrecision = 0;
        return malNoError as prMALError;
    }
    malNoError as prMALError
}

pub unsafe fn handle_query_param_summary(std_parms: *mut exportStdParms, param1: *mut c_void) -> prMALError {
    let summary_rec = param1 as *mut exParamSummaryRec;
    if summary_rec.is_null() || std_parms.is_null() {
        return malNoError as prMALError;
    }
    
    let sr = &mut *summary_rec;
    let std_p = &*std_parms;
    
    let mut width = 1920;
    let mut height = 1080;
    let mut fps_ticks: i64 = 4233600000;
    let mut sample_rate = 48000.0;
    let mut channels = 2;
    
    if let Some(get_basic_suite) = std_p.getSPBasicSuite {
        let basic_suite_ptr = get_basic_suite();
        if !basic_suite_ptr.is_null() {
            let basic_suite = &*(basic_suite_ptr as *const SPBasicSuite);
            if let Some(acquire_suite) = basic_suite.AcquireSuite {
                let mut param_suite_ptr: *const c_void = core::ptr::null();
                acquire_suite(kPrSDKExportParamSuite.as_ptr() as *const c_char, kPrSDKExportParamSuiteVersion as i32, &mut param_suite_ptr);
                if !param_suite_ptr.is_null() {
                    let param_suite = &*(param_suite_ptr as *const PrSDKExportParamSuite);
                    let mut val: exParamValues = core::mem::zeroed();
                    if let Some(get_param_value) = param_suite.GetParamValue {
                        get_param_value(sr.exporterPluginID, 0, ADBEVideoWidth.as_ptr() as *const c_char, &mut val);
                        width = val.value.__bindgen_anon_1.intValue;
                        get_param_value(sr.exporterPluginID, 0, ADBEVideoHeight.as_ptr() as *const c_char, &mut val);
                        height = val.value.__bindgen_anon_1.intValue;
                        get_param_value(sr.exporterPluginID, 0, ADBEVideoFPS.as_ptr() as *const c_char, &mut val);
                        fps_ticks = val.value.__bindgen_anon_1.timeValue;
                        get_param_value(sr.exporterPluginID, 0, ADBEAudioRatePerSecond.as_ptr() as *const c_char, &mut val);
                        sample_rate = val.value.__bindgen_anon_1.floatValue;
                        get_param_value(sr.exporterPluginID, 0, ADBEAudioNumChannels.as_ptr() as *const c_char, &mut val);
                        channels = val.value.__bindgen_anon_1.intValue;
                    }
                    if let Some(release_suite) = basic_suite.ReleaseSuite {
                        release_suite(kPrSDKExportParamSuite.as_ptr() as *const i8, kPrSDKExportParamSuiteVersion as i32);
                    }
                }
            }
        }
    }
    
    let fps = 254016000000.0 / (fps_ticks as f64);
    
    if sr.exportVideo != 0 {
        let video_str = format!("Video: AV1, {}x{}, {:.2} fps", width, height, fps);
        crate::str_to_utf16(&video_str, core::ptr::addr_of_mut!(sr.videoSummary) as *mut _, 256);
    }
    
    if sr.exportAudio != 0 {
        let ch_str = if channels == 1 { "Mono" } else if channels == 3 { "5.1" } else { "Stereo" };
        let audio_str = format!("Audio: {:.0} Hz, {}", sample_rate, ch_str);
        crate::str_to_utf16(&audio_str, core::ptr::addr_of_mut!(sr.audioSummary) as *mut _, 256);
    }
    
    return malNoError as prMALError;
}

pub unsafe fn handle_query_export_file_extension(std_parms: *mut exportStdParms, param1: *mut c_void) -> prMALError {
    let rec = param1 as *mut exQueryExportFileExtensionRec;
    if !rec.is_null() && !std_parms.is_null() {
        let rec = &mut *rec;
        let std_p = &*std_parms;
        
        let mut container_val = 0; // default mp4
        
        if let Some(get_basic_suite) = std_p.getSPBasicSuite {
            let basic_suite_ptr = get_basic_suite();
            if !basic_suite_ptr.is_null() {
                let basic_suite = &*(basic_suite_ptr as *const SPBasicSuite);
                if let Some(acquire_suite) = basic_suite.AcquireSuite {
                    let mut param_suite_ptr: *const c_void = core::ptr::null();
                    acquire_suite(kPrSDKExportParamSuite.as_ptr() as *const c_char, kPrSDKExportParamSuiteVersion as i32, &mut param_suite_ptr);
                    if !param_suite_ptr.is_null() {
                        let param_suite = &*(param_suite_ptr as *const PrSDKExportParamSuite);
                        let mut val: exParamValues = core::mem::zeroed();
                        if let Some(get_param_value) = param_suite.GetParamValue {
                            get_param_value(rec.exporterPluginID, 0, b"ADBEVideoContainer\0".as_ptr() as *const c_char, &mut val);
                            container_val = val.value.__bindgen_anon_1.intValue;
                        }
                        if let Some(release_suite) = basic_suite.ReleaseSuite {
                            release_suite(kPrSDKExportParamSuite.as_ptr() as *const i8, kPrSDKExportParamSuiteVersion as i32);
                        }
                    }
                }
            }
        }
        
        let file_type = rec.fileType;
        let nuka = u32::from_be_bytes(*b"NukA");
        let nuk8 = u32::from_be_bytes(*b"Nuk8");
        let nuk9 = u32::from_be_bytes(*b"Nuk9");
        
        // Match Filtered Container Choices supporting AV1/VP9:
        // 0 -> Matroska Video (.mkv)
        // 1 -> MPEG-4 (.mp4) (Regular, faststart)
        // 2 -> Hybrid MP4 (.mp4) (Fragmented empty_moov)
        // 3 -> Fragmented MP4 (.mp4)
        // 4 -> QuickTime (.mov) (Regular, faststart)
        // 5 -> Hybrid MOV (.mov) (Fragmented empty_moov)
        // 6 -> Fragmented MOV (.mov)
        let ext = match container_val {
            0 => "mkv",
            1 | 2 | 3 => "mp4",
            4 | 5 | 6 => "mov",
            _ => if file_type == nuka {
                "mp4"
            } else if file_type == nuk8 || file_type == nuk9 {
                "webm"
            } else {
                "mp4"
            }
        };
        
        crate::str_to_utf16(ext, core::ptr::addr_of_mut!(rec.outFileExtension) as *mut _, 256);
    }
    malNoError as prMALError
}

pub unsafe fn handle_query_export_color_space(std_parms: *mut exportStdParms, param1: *mut c_void) -> prMALError {
    let rec = param1 as *mut exQueryExportColorSpaceRec;
    if !rec.is_null() && !std_parms.is_null() {
        let rec = &mut *rec;
        let std_p = &*std_parms;
        
        let mut colorspace_choice = 0; // default Rec.709 8-bit
        
        if let Some(get_basic_suite) = std_p.getSPBasicSuite {
            let basic_suite_ptr = get_basic_suite();
            if !basic_suite_ptr.is_null() {
                let basic_suite = &*(basic_suite_ptr as *const SPBasicSuite);
                if let Some(acquire_suite) = basic_suite.AcquireSuite {
                    let mut param_suite_ptr: *const c_void = core::ptr::null();
                    acquire_suite(kPrSDKExportParamSuite.as_ptr() as *const c_char, kPrSDKExportParamSuiteVersion as i32, &mut param_suite_ptr);
                    if !param_suite_ptr.is_null() {
                        let param_suite = &*(param_suite_ptr as *const PrSDKExportParamSuite);
                        let mut val: exParamValues = core::mem::zeroed();
                        if let Some(get_param_value) = param_suite.GetParamValue {
                            get_param_value(rec.exporterPluginID, 0, b"NukeVideoColorSpace\0".as_ptr() as *const c_char, &mut val);
                            colorspace_choice = val.value.__bindgen_anon_1.intValue;
                        }
                        if let Some(release_suite) = basic_suite.ReleaseSuite {
                            release_suite(kPrSDKExportParamSuite.as_ptr() as *const i8, kPrSDKExportParamSuiteVersion as i32);
                        }
                    }
                }
            }
        }
        
        rec.outExportColorSpace.outColorSpaceType = kPrSDKColorSpaceType_Undefined as PrSDKColorSpaceType;
        
        let mut primaries = 1;
        let mut transfer = 1;
        let mut matrix = 1;
        let mut bitdepth = 8;
        
        if colorspace_choice == 1 { // Rec.2020 PQ
            primaries = 9;
            transfer = 16;
            matrix = 9;
            bitdepth = 10;
        } else if colorspace_choice == 2 { // Rec.2020 HLG
            primaries = 9;
            transfer = 18;
            matrix = 9;
            bitdepth = 10;
        }
        
        rec.outExportColorSpace.outSEICodesRec.colorPrimariesCode = primaries;
        rec.outExportColorSpace.outSEICodesRec.transferCharacteristicCode = transfer;
        rec.outExportColorSpace.outSEICodesRec.matrixEquationsCode = matrix;
        rec.outExportColorSpace.outSEICodesRec.bitDepth = bitdepth;
        rec.outExportColorSpace.outSEICodesRec.isFullRange = 0;
        rec.outExportColorSpace.outSEICodesRec.isRGB = 0;
        rec.outExportColorSpace.outSEICodesRec.isSceneReferred = 0;
    }
    malNoError as prMALError
}
