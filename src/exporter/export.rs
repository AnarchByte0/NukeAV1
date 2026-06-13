use std::ffi::c_void;
use std::os::raw::c_char;
use std::os::windows::ffi::OsStringExt;
use crate::*;

#[allow(unused_variables)]
fn log_debug(msg: &str) {
    #[cfg(debug_assertions)]
    {
        use std::fs::OpenOptions;
        use std::io::Write;
        let time = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs_f64())
            .unwrap_or(0.0);
        if let Ok(mut file) = OpenOptions::new()
            .create(true)
            .append(true)
            .open(std::env::temp_dir().join("NukeAV1_debug.log"))
        {
            let _ = writeln!(file, "[{:.4}] {}", time, msg);
        }
    }
}

pub unsafe fn handle_export(std_parms: *mut exportStdParms, param1: *mut c_void) -> prMALError {
    log_debug("--- handle_export STARTED ---");
    let mut aborted_flag = false;
    let export_rec = param1 as *mut exDoExportRec;
    if export_rec.is_null() {
        log_debug("Error: export_rec is null");
    }
    if std_parms.is_null() {
        log_debug("Error: std_parms is null");
    }
    
    if !export_rec.is_null() && !std_parms.is_null() {
        let export_rec = &mut *export_rec;
        let std_parms = &*std_parms;
        
        let export_audio_val = export_rec.exportAudio;
        let export_video_val = export_rec.exportVideo;
        let start_time_val = export_rec.startTime;
        let end_time_val = export_rec.endTime;
        log_debug(&format!("exportAudio: {}, exportVideo: {}", export_audio_val, export_video_val));
        log_debug(&format!("startTime: {}, endTime: {}", start_time_val, end_time_val));
        
        if let Some(get_basic_suite) = std_parms.getSPBasicSuite {
            let basic_suite_ptr = get_basic_suite();
            if basic_suite_ptr.is_null() {
                log_debug("Error: basic_suite_ptr is null");
            } else {
                let basic_suite = &*(basic_suite_ptr as *const SPBasicSuite);
                
                if let Some(acquire_suite) = basic_suite.AcquireSuite {
                    let mut param_suite_ptr: *const c_void = core::ptr::null();
                    let mut render_suite_ptr: *const c_void = core::ptr::null();
                    let mut audio_suite_ptr: *const c_void = core::ptr::null();
                    let mut progress_suite_ptr: *const c_void = core::ptr::null();
                    let mut info_suite_ptr: *const c_void = core::ptr::null();
                    
                    acquire_suite(kPrSDKExportParamSuite.as_ptr() as *const c_char, kPrSDKExportParamSuiteVersion as i32, &mut param_suite_ptr);
                    acquire_suite(kPrSDKSequenceRenderSuite.as_ptr() as *const c_char, kPrSDKSequenceRenderSuiteVersion as i32, &mut render_suite_ptr);
                    acquire_suite(kPrSDKSequenceAudioSuite.as_ptr() as *const c_char, kPrSDKSequenceAudioSuiteVersion as i32, &mut audio_suite_ptr);
                    acquire_suite(kPrSDKExportProgressSuite.as_ptr() as *const c_char, kPrSDKExportProgressSuiteVersion as i32, &mut progress_suite_ptr);
                    acquire_suite(kPrSDKExportInfoSuite.as_ptr() as *const c_char, kPrSDKExportInfoSuiteVersion as i32, &mut info_suite_ptr);
                    
                    log_debug(&format!("param_suite_ptr: {:?}, render_suite_ptr: {:?}, audio_suite_ptr: {:?}, progress_suite_ptr: {:?}, info_suite_ptr: {:?}", param_suite_ptr, render_suite_ptr, audio_suite_ptr, progress_suite_ptr, info_suite_ptr));
                    
                    // Query timeline width and height as fallback
                    let mut timeline_width = 1920;
                    let mut timeline_height = 1080;
                    if !info_suite_ptr.is_null() {
                        let info_suite = &*(info_suite_ptr as *const PrSDKExportInfoSuite);
                        if let Some(get_src_info) = info_suite.GetExportSourceInfo {
                            let mut info_val: PrParam = core::mem::zeroed();
                            if get_src_info(export_rec.exporterPluginID, PrExportSourceInfoSelector_kExportInfo_VideoWidth, &mut info_val) == 0 {
                                timeline_width = info_val.__bindgen_anon_1.mInt32;
                            }
                            if get_src_info(export_rec.exporterPluginID, PrExportSourceInfoSelector_kExportInfo_VideoHeight, &mut info_val) == 0 {
                                timeline_height = info_val.__bindgen_anon_1.mInt32;
                            }
                        }
                    }

                    if !param_suite_ptr.is_null() && !render_suite_ptr.is_null() {
                        let param_suite = &*(param_suite_ptr as *const PrSDKExportParamSuite);
                        let render_suite = &*(render_suite_ptr as *const PrSDKSequenceRenderSuite);
                        
                        // 1. Get FPS
                        let mut val: exParamValues = core::mem::zeroed();
                        if let Some(get_param_value) = param_suite.GetParamValue {
                            get_param_value(export_rec.exporterPluginID, 0, ADBEVideoFPS.as_ptr() as *const c_char, &mut val);
                        }
                        let ticks_per_frame = val.value.__bindgen_anon_1.timeValue;
                        log_debug(&format!("ticks_per_frame: {}", ticks_per_frame));
                        
                        // 2. Init Renderer
                        let mut video_render_id: csSDK_uint32 = 0;
                        if let Some(make_renderer) = render_suite.MakeVideoRenderer {
                            let err = make_renderer(export_rec.exporterPluginID, &mut video_render_id, ticks_per_frame);
                            log_debug(&format!("MakeVideoRenderer result: {}, render_id: {}", err, video_render_id));
                        }
                        
                        // 3. Acquire File Suite & PPix Suite
                        let mut file_suite_ptr: *const c_void = core::ptr::null();
                        let mut ppix_suite_ptr: *const c_void = core::ptr::null();
                        acquire_suite(kPrSDKExportFileSuite.as_ptr() as *const c_char, kPrSDKExportFileSuiteVersion as i32, &mut file_suite_ptr);
                        acquire_suite(kPrSDKPPixSuite.as_ptr() as *const c_char, kPrSDKPPixSuiteVersion as i32, &mut ppix_suite_ptr);
                        
                        log_debug(&format!("file_suite_ptr: {:?}, ppix_suite_ptr: {:?}", file_suite_ptr, ppix_suite_ptr));
                        
                        if !file_suite_ptr.is_null() && !ppix_suite_ptr.is_null() {
                            let file_suite = &*(file_suite_ptr as *const PrSDKExportFileSuite);
                            let ppix_suite = &*(ppix_suite_ptr as *const PrSDKPPixSuite);
                            
                            // Get Parameters for FFmpeg
                            let mut val: exParamValues = core::mem::zeroed();
                            let codec_choice = if export_rec.fileType == u32::from_be_bytes(*b"NukA") {
                                0 // AV1
                            } else if export_rec.fileType == u32::from_be_bytes(*b"Nuk9") {
                                1 // VP9
                            } else {
                                2 // VP8
                            };
                            let mut encoder_choice = 0;
                            let mut width = 1920;
                            let mut height = 1080;
                            let mut target_bitrate = 10.0;
                            let mut sample_rate = 48000.0;
                            let mut channels = 2; // stereo
                            let mut container_choice = 0; // mp4
                            let mut multiplexer_choice = 0; // MP4
                            let mut bitrate_mode = 0; // default VBR
                            let mut max_bitrate = 12.0;
                            let mut audio_bitrate_choice = 1; // default 128 kbps
                            let mut gop_size = 33;
                            let mut profile = 0;
                            let mut level = 0;
                            let mut tier = 0;
                            let mut colorspace = 0;
                            
                            let mut preset = 4;
                            let mut tuning = 0;
                            let mut multipass = 1;
                            let mut lookahead = true;
                            let mut spatial_aq = true;
                            let mut bframes = 2;
                            let mut bframe_ref = 0;

                            if let Some(get_param_value) = param_suite.GetParamValue {
                                get_param_value(export_rec.exporterPluginID, 0, b"NukeAV1Encoder\0".as_ptr() as *const c_char, &mut val);
                                encoder_choice = val.value.__bindgen_anon_1.intValue;
                                get_param_value(export_rec.exporterPluginID, 0, ADBEVideoWidth.as_ptr() as *const c_char, &mut val);
                                width = val.value.__bindgen_anon_1.intValue;
                                get_param_value(export_rec.exporterPluginID, 0, ADBEVideoHeight.as_ptr() as *const c_char, &mut val);
                                height = val.value.__bindgen_anon_1.intValue;
                                get_param_value(export_rec.exporterPluginID, 0, ADBEVideoTargetBitrate.as_ptr() as *const c_char, &mut val);
                                target_bitrate = val.value.__bindgen_anon_1.floatValue;
                                get_param_value(export_rec.exporterPluginID, 0, ADBEVideoMaxBitrate.as_ptr() as *const c_char, &mut val);
                                max_bitrate = val.value.__bindgen_anon_1.floatValue;
                                get_param_value(export_rec.exporterPluginID, 0, ADBEVideoBitrateEncoding.as_ptr() as *const c_char, &mut val);
                                bitrate_mode = val.value.__bindgen_anon_1.intValue;

                                // Read OBS options
                                get_param_value(export_rec.exporterPluginID, 0, b"NukePreset\0".as_ptr() as *const c_char, &mut val);
                                preset = val.value.__bindgen_anon_1.intValue;
                                get_param_value(export_rec.exporterPluginID, 0, b"NukeTuning\0".as_ptr() as *const c_char, &mut val);
                                tuning = val.value.__bindgen_anon_1.intValue;
                                get_param_value(export_rec.exporterPluginID, 0, b"NukeMultipass\0".as_ptr() as *const c_char, &mut val);
                                multipass = val.value.__bindgen_anon_1.intValue;
                                get_param_value(export_rec.exporterPluginID, 0, b"NukeLookAhead\0".as_ptr() as *const c_char, &mut val);
                                lookahead = val.value.__bindgen_anon_1.intValue != 0;
                                get_param_value(export_rec.exporterPluginID, 0, b"NukeAdaptiveQuant\0".as_ptr() as *const c_char, &mut val);
                                spatial_aq = val.value.__bindgen_anon_1.intValue != 0;
                                get_param_value(export_rec.exporterPluginID, 0, b"NukeBFrames\0".as_ptr() as *const c_char, &mut val);
                                bframes = val.value.__bindgen_anon_1.intValue;
                                get_param_value(export_rec.exporterPluginID, 0, b"NukeBFrameRef\0".as_ptr() as *const c_char, &mut val);
                                bframe_ref = val.value.__bindgen_anon_1.intValue;
                                
                                get_param_value(export_rec.exporterPluginID, 0, ADBEAudioRatePerSecond.as_ptr() as *const c_char, &mut val);
                                sample_rate = val.value.__bindgen_anon_1.floatValue;
                                get_param_value(export_rec.exporterPluginID, 0, ADBEAudioNumChannels.as_ptr() as *const c_char, &mut val);
                                channels = val.value.__bindgen_anon_1.intValue;
                                get_param_value(export_rec.exporterPluginID, 0, ADBEAudioBitrate.as_ptr() as *const c_char, &mut val);
                                audio_bitrate_choice = val.value.__bindgen_anon_1.intValue;

                                get_param_value(export_rec.exporterPluginID, 0, b"ADBEVideoContainer\0".as_ptr() as *const c_char, &mut val);
                                container_choice = val.value.__bindgen_anon_1.intValue;
                                get_param_value(export_rec.exporterPluginID, 0, b"ADBEExporterMultiplexerDropdown\0".as_ptr() as *const c_char, &mut val);
                                multiplexer_choice = val.value.__bindgen_anon_1.intValue;
                                get_param_value(export_rec.exporterPluginID, 0, b"NukeVideoColorSpace\0".as_ptr() as *const c_char, &mut val);
                                colorspace = val.value.__bindgen_anon_1.intValue;

                                // Codec-specific parameters
                                if codec_choice == 0 { // AV1
                                    get_param_value(export_rec.exporterPluginID, 0, b"NukeAV1KeyFrame\0".as_ptr() as *const c_char, &mut val);
                                    gop_size = val.value.__bindgen_anon_1.floatValue as i32;
                                    get_param_value(export_rec.exporterPluginID, 0, b"NukeAV1Profile\0".as_ptr() as *const c_char, &mut val);
                                    profile = val.value.__bindgen_anon_1.intValue;
                                    get_param_value(export_rec.exporterPluginID, 0, b"NukeAV1Level\0".as_ptr() as *const c_char, &mut val);
                                    level = val.value.__bindgen_anon_1.intValue;
                                    get_param_value(export_rec.exporterPluginID, 0, b"NukeAV1Tier\0".as_ptr() as *const c_char, &mut val);
                                    tier = val.value.__bindgen_anon_1.intValue;
                                } else if codec_choice == 1 { // VP9
                                    get_param_value(export_rec.exporterPluginID, 0, b"NukeVP9KeyFrame\0".as_ptr() as *const c_char, &mut val);
                                    gop_size = val.value.__bindgen_anon_1.floatValue as i32;
                                    get_param_value(export_rec.exporterPluginID, 0, b"NukeVP9Profile\0".as_ptr() as *const c_char, &mut val);
                                    profile = val.value.__bindgen_anon_1.intValue;
                                } else if codec_choice == 2 { // VP8
                                    get_param_value(export_rec.exporterPluginID, 0, b"NukeVP8KeyFrame\0".as_ptr() as *const c_char, &mut val);
                                    gop_size = val.value.__bindgen_anon_1.floatValue as i32;
                                }
                            }
                            
                            let audio_bitrate = match audio_bitrate_choice {
                                0 => 96000,
                                1 => 128000,
                                2 => 160000,
                                3 => 192000,
                                4 => 256000,
                                5 => 320000,
                                _ => 128000,
                            };
                            
                            log_debug(&format!("params: codec: {}, enc: {}, size: {}x{}, br: {}, sr: {}, ch: {}, container: {}", codec_choice, encoder_choice, width, height, target_bitrate, sample_rate, channels, container_choice));
                            
                             // Hardware/software prioritisation for Auto (0, 0)
                             let vcodec = match (codec_choice, encoder_choice) {
                                 (0, 0) => {
                                     // Priority for AV1 Auto selection: NVENC -> AMF -> QSV -> CPU
                                     if crate::ffmpeg_ffi::avcodec_find_encoder_by_name(b"av1_nvenc\0".as_ptr() as *const c_char).is_null() == false {
                                         "av1_nvenc"
                                     } else if crate::ffmpeg_ffi::avcodec_find_encoder_by_name(b"av1_amf\0".as_ptr() as *const c_char).is_null() == false {
                                         "av1_amf"
                                     } else if crate::ffmpeg_ffi::avcodec_find_encoder_by_name(b"av1_qsv\0".as_ptr() as *const c_char).is_null() == false {
                                         "av1_qsv"
                                     } else {
                                         "libaom_av1"
                                     }
                                 },
                                 (0, 1) => "libaom_av1", // CPU
                                 (0, 2) => "av1_nvenc", // Nvidia
                                 (0, 3) => "av1_amf",   // AMD
                                 (0, 4) => "av1_qsv",   // Intel
                                 (1, 0) => {
                                     // Priority for VP9 Auto selection: NVENC -> QSV -> CPU
                                     if crate::ffmpeg_ffi::avcodec_find_encoder_by_name(b"vp9_nvenc\0".as_ptr() as *const c_char).is_null() == false {
                                         "vp9_nvenc"
                                     } else if crate::ffmpeg_ffi::avcodec_find_encoder_by_name(b"vp9_qsv\0".as_ptr() as *const c_char).is_null() == false {
                                         "vp9_qsv"
                                     } else {
                                         "libvpx-vp9"
                                     }
                                 },
                                 (1, 1) => "libvpx-vp9",// CPU
                                 (1, 2) => "vp9_nvenc", // Nvidia
                                 (1, 3) => "libvpx-vp9",// AMD (fallback to CPU if AMF VP9 is unavailable)
                                 (1, 4) => "vp9_qsv",   // Intel
                                 (2, _) => "libvpx",    // VP8
                                 _ => "libaom_av1",
                             };

                            let audio_codec = if container_choice == 0 {
                                // Matroska Video (.mkv) supports Opus
                                "opus"
                            } else {
                                "aac"
                            };

                            let audio_enabled = export_rec.exportAudio != 0;
                            let mut audio_render_id: csSDK_uint32 = 0;
                            let mut audio_render_initialized = false;

                            if audio_enabled && !audio_suite_ptr.is_null() {
                                let audio_suite = &*(audio_suite_ptr as *const PrSDKSequenceAudioSuite);
                                let mut channel_labels = match channels {
                                    1 => vec![PrAudioChannelLabel_kPrAudioChannelLabel_FrontCenter],
                                    2 => vec![
                                        PrAudioChannelLabel_kPrAudioChannelLabel_FrontLeft,
                                        PrAudioChannelLabel_kPrAudioChannelLabel_FrontRight,
                                    ],
                                    6 => vec![
                                        PrAudioChannelLabel_kPrAudioChannelLabel_FrontLeft,
                                        PrAudioChannelLabel_kPrAudioChannelLabel_FrontRight,
                                        PrAudioChannelLabel_kPrAudioChannelLabel_FrontCenter,
                                        PrAudioChannelLabel_kPrAudioChannelLabel_LowFrequency,
                                        PrAudioChannelLabel_kPrAudioChannelLabel_BackLeft,
                                        PrAudioChannelLabel_kPrAudioChannelLabel_BackRight,
                                    ],
                                    _ => vec![PrAudioChannelLabel_kPrAudioChannelLabel_Discrete; channels as usize],
                                };

                                if let Some(make_audio_renderer) = audio_suite.MakeAudioRenderer {
                                    let err = make_audio_renderer(
                                        export_rec.exporterPluginID,
                                        export_rec.startTime,
                                        channels as u32,
                                        channel_labels.as_mut_ptr(),
                                        PrAudioSampleType_kPrAudioSampleType_32BitFloat,
                                        sample_rate as f32,
                                        &mut audio_render_id,
                                    );
                                    log_debug(&format!("MakeAudioRenderer result: {}, audio_render_id: {}", err, audio_render_id));
                                    if err == 0 {
                                        audio_render_initialized = true;
                                    }
                                }
                            }
                            
                            // Fallback if width/height are set to -1 (Custom option in presets)
                            // In this case, we read the sequence width/height from the queried timeline info
                            if width <= 0 {
                                width = timeline_width;
                            }
                            if height <= 0 {
                                height = timeline_height;
                            }

                            let fps = if ticks_per_frame > 0 { 254016000000.0 / (ticks_per_frame as f64) } else { 30.0 };
                            
                            // Get Platform Path
                            let mut path_len: csSDK_int32 = 0;
                            if let Some(get_path) = file_suite.GetPlatformPath {
                                get_path(export_rec.fileObject, &mut path_len, core::ptr::null_mut());
                                log_debug(&format!("GetPlatformPath path_len: {}", path_len));
                                if path_len > 0 {
                                    let mut utf16_path: Vec<u16> = vec![0; path_len as usize];
                                    get_path(export_rec.fileObject, &mut path_len, utf16_path.as_mut_ptr());
                                    
                                    let os_string = std::ffi::OsString::from_wide(&utf16_path[..path_len as usize - 1]);
                                    if let Ok(output_path) = os_string.into_string() {
                                        log_debug(&format!("output_path: {}", output_path));
                                        
                                        match crate::exporter::ffmpeg::FFmpegEncoder::new(
                                            &output_path,
                                            vcodec,
                                            width as i32,
                                            height as i32,
                                            fps as i32,
                                            (target_bitrate * 1000000.0) as i64,
                                            audio_render_initialized && multiplexer_choice != 2,
                                            audio_codec,
                                            sample_rate as i32,
                                            channels as i32,
                                            bitrate_mode,
                                            (max_bitrate * 1000000.0) as i64,
                                            audio_bitrate,
                                            gop_size,
                                            profile,
                                            level,
                                            tier,
                                            colorspace,
                                            
                                            // OBS-like options
                                            preset,
                                            tuning,
                                            multipass,
                                            lookahead,
                                            spatial_aq,
                                            bframes,
                                            bframe_ref,
                                            container_choice,
                                        ) {
                                            Ok(mut encoder) => {
                                                log_debug("FFmpegEncoder initialized successfully");
                                                // Render Loop
                                                let mut current_time = export_rec.startTime;
                                                let pixel_format = if colorspace == 1 || colorspace == 2 {
                                                    PrPixelFormat_PrPixelFormat_BGRA_4444_32f
                                                } else {
                                                    PrPixelFormat_PrPixelFormat_BGRA_4444_8u
                                                };
                                                let pixel_formats = [pixel_format];
                                                let mut frame_index = 0;
                                                let mut total_audio_samples_requested = 0;
                                                let mut export_aborted = false;

                                                let total_frames = if ticks_per_frame > 0 {
                                                    ((export_rec.endTime - export_rec.startTime) as f64 / ticks_per_frame as f64).ceil() as usize
                                                } else {
                                                    1
                                                };
                                                let start_instant = std::time::Instant::now();
                                                log_debug(&format!("Starting render loop. total_frames: {}", total_frames));

                                                let progress_suite = if !progress_suite_ptr.is_null() {
                                                    Some(&*(progress_suite_ptr as *const PrSDKExportProgressSuite))
                                                } else {
                                                    None
                                                };

                                                // Initialize progress bar immediately at 1% to force Premiere Pro UI to update and display text
                                                if let Some(suite) = progress_suite {
                                                    let init_str = "Rem: Calculating... | Size: 0.0 MB (Est: Calculating...)";
                                                    let mut progress_utf16 = [0u16; 256];
                                                    crate::str_to_utf16(init_str, progress_utf16.as_mut_ptr(), 256);
                                                    if let Some(set_str) = suite.SetProgressString {
                                                        set_str(export_rec.exporterPluginID, progress_utf16.as_mut_ptr());
                                                    }
                                                    if let Some(update_percent) = suite.UpdateProgressPercent {
                                                        let _ = update_percent(export_rec.exporterPluginID, 0.01);
                                                    }
                                                }
                                                
                                                while current_time < export_rec.endTime {
                                                    let mut render_params: SequenceRender_ParamsRec = core::mem::zeroed();
                                                    render_params.inRequestedPixelFormatArray = pixel_formats.as_ptr();
                                                    render_params.inRequestedPixelFormatArrayCount = 1;
                                                    render_params.inWidth = width;
                                                    render_params.inHeight = height;
                                                    render_params.inPixelAspectRatioNumerator = 1;
                                                    render_params.inPixelAspectRatioDenominator = 1;
                                                    render_params.inRenderQuality = PrRenderQuality_kPrRenderQuality_High;
                                                    render_params.inDeinterlaceQuality = PrRenderQuality_kPrRenderQuality_High;
                                                    
                                                    let mut get_frame_return: SequenceRender_GetFrameReturnRec = core::mem::zeroed();
                                                    
                                                    if let Some(render_frame) = render_suite.RenderVideoFrame {
                                                        if frame_index < 5 || frame_index % 1000 == 0 {
                                                            log_debug(&format!("Rendering frame {}...", frame_index));
                                                        }
                                                        let err = render_frame(video_render_id, current_time, &mut render_params, 0, &mut get_frame_return);
                                                        if frame_index < 5 || frame_index % 1000 == 0 {
                                                            log_debug(&format!("RenderVideoFrame returned {} for frame {}", err, frame_index));
                                                        }
                                                        
                                                        if err == 0 && !get_frame_return.outFrame.is_null() {
                                                            let mut pixel_ptr: *mut c_char = core::ptr::null_mut();
                                                            if let Some(get_pixels) = ppix_suite.GetPixels {
                                                                get_pixels(get_frame_return.outFrame, PrPPixBufferAccess_PrPPixBufferAccess_ReadOnly, &mut pixel_ptr);
                                                            }
                                                            
                                                            let mut row_bytes = 0;
                                                            if let Some(get_row_bytes) = ppix_suite.GetRowBytes {
                                                                get_row_bytes(get_frame_return.outFrame, &mut row_bytes);
                                                            }
                                                            
                                                            if !pixel_ptr.is_null() {
                                                                let frame_size = (row_bytes * height) as usize;
                                                                let pixel_slice = core::slice::from_raw_parts(pixel_ptr as *const u8, frame_size);
                                                                
                                                                let mut audio_samples: Option<Vec<Vec<f32>>> = None;
                                                                frame_index += 1;
 
                                                                if audio_render_initialized {
                                                                    let expected_total_audio_samples = ((frame_index as f64 / fps) * sample_rate).round() as u64;
                                                                    let samples_to_request = expected_total_audio_samples - total_audio_samples_requested;
 
                                                                    if samples_to_request > 0 {
                                                                        let audio_suite = &*(audio_suite_ptr as *const PrSDKSequenceAudioSuite);
                                                                        let mut channel_buffers = vec![vec![0.0f32; samples_to_request as usize]; channels as usize];
                                                                        let mut channel_ptrs: Vec<*mut f32> = channel_buffers.iter_mut().map(|buf| buf.as_mut_ptr()).collect();
 
                                                                        if let Some(get_audio) = audio_suite.GetAudio {
                                                                            let err = get_audio(audio_render_id, samples_to_request as csSDK_uint32, channel_ptrs.as_mut_ptr(), 0);
                                                                            if err == 0 {
                                                                                audio_samples = Some(channel_buffers);
                                                                                total_audio_samples_requested += samples_to_request;
                                                                            }
                                                                        }
                                                                    }
                                                                }
 
                                                                if let Err(e) = encoder.encode_frame(Some((pixel_slice, row_bytes)), audio_samples.as_deref()) {
                                                                    log_debug(&format!("Error during encode_frame: {}", e));
                                                                }

                                                                if let Some(suite) = progress_suite {
                                                                    let progress_val = (frame_index as f32 / total_frames as f32).min(1.0).max(0.0);
                                                                    
                                                                    // Estimate file size
                                                                    let current_bytes = std::fs::metadata(&output_path).map(|m| m.len()).unwrap_or(0);
                                                                    let est_bytes = if frame_index > 0 {
                                                                        (current_bytes as f64 / frame_index as f64) * total_frames as f64
                                                                    } else {
                                                                        0.0
                                                                    } as u64;
                                                                    
                                                                    // Calculate times
                                                                    let elapsed_secs = start_instant.elapsed().as_secs_f64();
                                                                    let remaining_secs = if frame_index > 0 {
                                                                        (elapsed_secs / frame_index as f64) * (total_frames - frame_index) as f64
                                                                    } else {
                                                                        0.0
                                                                    };
                                                                    
                                                                    let format_duration = |secs: f64| -> String {
                                                                        let s = secs.round() as u64;
                                                                        let h = s / 3600;
                                                                        let m = (s % 3600) / 60;
                                                                        let sec = s % 60;
                                                                        if h > 0 {
                                                                            format!("{:02}:{:02}:{:02}", h, m, sec)
                                                                        } else {
                                                                            format!("{:02}:{:02}", m, sec)
                                                                        }
                                                                    };
                                                                    
                                                                    let format_size = |bytes: u64| -> String {
                                                                        let mb = bytes as f64 / (1024.0 * 1024.0);
                                                                        let gb = mb / 1024.0;
                                                                        if gb >= 1.0 {
                                                                            format!("{:.2} GB", gb)
                                                                        } else if mb >= 1.0 {
                                                                            format!("{:.1} MB", mb)
                                                                        } else {
                                                                            format!("{:.1} KB", bytes as f64 / 1024.0)
                                                                        }
                                                                    };

                                                                    let remaining_str = format_duration(remaining_secs);
                                                                    let cur_size_str = format_size(current_bytes);
                                                                    let est_size_str = format_size(est_bytes);
                                                                    
                                                                    // Avoid \n since Premiere's progress box is single-line and clips/ignores newlines
                                                                    let progress_str = format!(
                                                                        "Rem: {} | Size: {} (Est: {})",
                                                                        remaining_str,
                                                                        cur_size_str,
                                                                        est_size_str
                                                                    );
                                                                    
                                                                    let mut progress_utf16 = [0u16; 256];
                                                                    crate::str_to_utf16(&progress_str, progress_utf16.as_mut_ptr(), 256);
                                                                    
                                                                    if let Some(set_str) = suite.SetProgressString {
                                                                        set_str(export_rec.exporterPluginID, progress_utf16.as_mut_ptr());
                                                                    }
                                                                    
                                                                    if let Some(update_percent) = suite.UpdateProgressPercent {
                                                                        let pr_err = update_percent(export_rec.exporterPluginID, progress_val);
                                                                        if pr_err != 0 {
                                                                            log_debug(&format!("Progress update returned code: {}", pr_err));
                                                                            if pr_err == PrExportReturnValue_exportReturn_Abort as prSuiteError {
                                                                                log_debug("User cancelled export.");
                                                                                export_aborted = true;
                                                                            }
                                                                        }
                                                                    }
                                                                }
                                                            }
                                                            
                                                            if let Some(dispose) = ppix_suite.Dispose {
                                                                dispose(get_frame_return.outFrame);
                                                            }
                                                        } else {
                                                            log_debug(&format!("RenderVideoFrame failed or outFrame is null: error {}", err));
                                                        }
                                                    }
                                                    if export_aborted {
                                                        break;
                                                    }
                                                    current_time += ticks_per_frame;
                                                }
                                                log_debug(&format!("Render loop finished. Total frames: {}", frame_index));
                                                if let Err(e) = encoder.finish() {
                                                    log_debug(&format!("Error during encoder.finish: {}", e));
                                                } else {
                                                    log_debug("Encoder finished and trailer written successfully");
                                                }
                                                
                                                if export_aborted {
                                                    aborted_flag = true;
                                                }
                                                
                                                if audio_render_initialized {
                                                    let audio_suite = &*(audio_suite_ptr as *const PrSDKSequenceAudioSuite);
                                                    if let Some(release_audio) = audio_suite.ReleaseAudioRenderer {
                                                        release_audio(export_rec.exporterPluginID, audio_render_id);
                                                    }
                                                }
                                            }
                                            Err(e) => {
                                                log_debug(&format!("FFmpegEncoder::new failed: {}", e));
                                            }
                                        }
                                    }
                                }
                            }
                            
                            if let Some(release_suite) = basic_suite.ReleaseSuite {
                                release_suite(kPrSDKExportFileSuite.as_ptr() as *const c_char, kPrSDKExportFileSuiteVersion as i32);
                                release_suite(kPrSDKPPixSuite.as_ptr() as *const c_char, kPrSDKPPixSuiteVersion as i32);
                            }
                        }
                        
                        // 4. Release Renderer
                        if let Some(release_renderer) = render_suite.ReleaseVideoRenderer {
                            release_renderer(export_rec.exporterPluginID, video_render_id);
                        }
                    }
                    
                    if let Some(release_suite) = basic_suite.ReleaseSuite {
                        release_suite(kPrSDKExportParamSuite.as_ptr() as *const c_char, kPrSDKExportParamSuiteVersion as i32);
                        release_suite(kPrSDKSequenceRenderSuite.as_ptr() as *const c_char, kPrSDKSequenceRenderSuiteVersion as i32);
                        if !audio_suite_ptr.is_null() {
                            release_suite(kPrSDKSequenceAudioSuite.as_ptr() as *const c_char, kPrSDKSequenceAudioSuiteVersion as i32);
                        }
                        if !progress_suite_ptr.is_null() {
                            release_suite(kPrSDKExportProgressSuite.as_ptr() as *const c_char, kPrSDKExportProgressSuiteVersion as i32);
                        }
                        if !info_suite_ptr.is_null() {
                            release_suite(kPrSDKExportInfoSuite.as_ptr() as *const c_char, kPrSDKExportInfoSuiteVersion as i32);
                        }
                    }
                }
            }
        }
    }
    log_debug("--- handle_export FINISHED ---");
    if aborted_flag {
        PrExportReturnValue_exportReturn_Abort as prMALError
    } else {
        malNoError as prMALError
    }
}
