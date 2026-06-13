use std::ffi::c_void;
use crate::*;

pub unsafe fn handle_generate_default_params(std_parms: *mut exportStdParms, param1: *mut c_void) -> prMALError {
    let param_rec = param1 as *mut exGenerateDefaultParamRec;
    if !param_rec.is_null() && !std_parms.is_null() {
        let param_rec = &mut *param_rec;
        let std_parms = &mut *std_parms;
        
        if let Some(get_suite) = std_parms.getSPBasicSuite {
            let basic_suite = get_suite();
            if !basic_suite.is_null() {
                let basic_suite = &*basic_suite;
                
                if let Some(acquire_suite) = basic_suite.AcquireSuite {
                    let mut param_suite_ptr: *const core::ffi::c_void = core::ptr::null();
                    acquire_suite(
                        kPrSDKExportParamSuite.as_ptr() as *const i8,
                        kPrSDKExportParamSuiteVersion as i32,
                        &mut param_suite_ptr,
                    );
                    
                    if !param_suite_ptr.is_null() {
                        let param_suite = &*(param_suite_ptr as *const PrSDKExportParamSuite);
                        
                        if let Some(builder) = crate::utils::UIBuilder::new(param_suite, param_rec.exporterPluginID) {
                            // 1. Video Settings
                            builder.add_group(ADBETopParamGroup, ADBEVideoTabGroup, "Video");
                            builder.add_group(ADBEVideoTabGroup, ADBEBasicVideoGroup, "Basic Video Settings");

                            // Frame Size presets using width and height as constrained value pairs
                            builder.add_int_param(ADBEBasicVideoGroup, ADBEVideoWidth, "Frame Width", 1920, 16, 8192);
                            builder.add_dropdown_item(ADBEVideoWidth, 4096, "4K (4096)");
                            builder.add_dropdown_item(ADBEVideoWidth, 3840, "UHD (3840)");
                            builder.add_dropdown_item(ADBEVideoWidth, 2560, "Quad HD (2560)");
                            builder.add_dropdown_item(ADBEVideoWidth, 1920, "Full HD (1920)");
                            builder.add_dropdown_item(ADBEVideoWidth, 1280, "HD (1280)");
                            builder.add_dropdown_item(ADBEVideoWidth, 854, "SD NTSC Wide (854)");
                            builder.add_dropdown_item(ADBEVideoWidth, 720, "SD NTSC (720)");
                            builder.add_dropdown_item(ADBEVideoWidth, -1, "Custom");

                            builder.add_int_param(ADBEBasicVideoGroup, ADBEVideoHeight, "Frame Height", 1080, 16, 8192);
                            builder.add_dropdown_item(ADBEVideoHeight, 2160, "4K / UHD (2160)");
                            builder.add_dropdown_item(ADBEVideoHeight, 1440, "Quad HD (1440)");
                            builder.add_dropdown_item(ADBEVideoHeight, 1080, "Full HD (1080)");
                            builder.add_dropdown_item(ADBEVideoHeight, 720, "HD (720)");
                            builder.add_dropdown_item(ADBEVideoHeight, 480, "SD NTSC (480)");
                            builder.add_dropdown_item(ADBEVideoHeight, -1, "Custom");
                            
                            // Frame rate
                            builder.add_time_dropdown(ADBEBasicVideoGroup, ADBEVideoFPS, "Frame Rate", 4233600000); // default 60 fps
                            builder.add_dropdown_item_time(ADBEVideoFPS, 10594627200, "23.976");
                            builder.add_dropdown_item_time(ADBEVideoFPS, 10584000000, "24");
                            builder.add_dropdown_item_time(ADBEVideoFPS, 10160640000, "25");
                            builder.add_dropdown_item_time(ADBEVideoFPS, 8475283200, "29.97");
                            builder.add_dropdown_item_time(ADBEVideoFPS, 8467200000, "30");
                            builder.add_dropdown_item_time(ADBEVideoFPS, 5080320000, "50");
                            builder.add_dropdown_item_time(ADBEVideoFPS, 4237641600, "59.94");
                            builder.add_dropdown_item_time(ADBEVideoFPS, 4233600000, "60");
                            builder.add_dropdown_item_time(ADBEVideoFPS, 2822400000, "90");
                            builder.add_dropdown_item_time(ADBEVideoFPS, 2540160000, "100");
                            builder.add_dropdown_item_time(ADBEVideoFPS, 2118820800, "119.88");
                            builder.add_dropdown_item_time(ADBEVideoFPS, 2116800000, "120");
                            builder.add_dropdown_item_time(ADBEVideoFPS, 1764000000, "144");
                            builder.add_dropdown_item_time(ADBEVideoFPS, 1058400000, "240");
                            builder.add_dropdown_item_time(ADBEVideoFPS, 846720000, "300");
                            builder.add_dropdown_item_time(ADBEVideoFPS, -1, "Custom");
                            
                            // PAR presets as ratio dropdown items
                            builder.add_ratio_param(ADBEBasicVideoGroup, ADBEVideoAspect, "Pixel Aspect Ratio", 1, 1);
                            builder.add_dropdown_item_ratio(ADBEVideoAspect, 1, 1, "Square Pixels (1.0)");
                            builder.add_dropdown_item_ratio(ADBEVideoAspect, 10, 11, "D1/DV NTSC (0.9091)");
                            builder.add_dropdown_item_ratio(ADBEVideoAspect, 40, 33, "D1/DV NTSC Widescreen 16:9 (1.2121)");
                            builder.add_dropdown_item_ratio(ADBEVideoAspect, 59, 54, "D1/DV PAL (1.0940)");
                            builder.add_dropdown_item_ratio(ADBEVideoAspect, 118, 81, "D1/DV PAL Widescreen 16:9 (1.4587)");
                            builder.add_dropdown_item_ratio(ADBEVideoAspect, 2, 1, "Anamorphic 2:1 (2.0)");
                            builder.add_dropdown_item_ratio(ADBEVideoAspect, 4, 3, "HD Anamorphic 1080 (1.333)");
                            builder.add_dropdown_item_ratio(ADBEVideoAspect, 3, 2, "DVCPRO HD (1.5)");
                            builder.add_dropdown_item_ratio(ADBEVideoAspect, -1, -1, "Custom");
                            
                            // Field type (Progressive)
                            builder.add_dropdown(ADBEBasicVideoGroup, ADBEVideoFieldType, "Field Order", 0); // progressive
                            builder.add_dropdown_item(ADBEVideoFieldType, 0, "Progressive");
                            
                            // Container Format
                            let container_id = b"ADBEVideoContainer\0";
                            builder.add_dropdown(ADBEBasicVideoGroup, container_id, "File Extension", 2); // default Hybrid MP4 (now index 2)
                            builder.add_dropdown_item(container_id, 0, "Matroska Video (.mkv)");
                            builder.add_dropdown_item(container_id, 1, "MPEG-4 (.mp4)");
                            builder.add_dropdown_item(container_id, 2, "Hybrid MP4 (.mp4)");
                            builder.add_dropdown_item(container_id, 3, "Fragmented MP4 (.mp4)");
                            builder.add_dropdown_item(container_id, 4, "QuickTime (.mov)");
                            builder.add_dropdown_item(container_id, 5, "Hybrid MOV (.mov)");
                            builder.add_dropdown_item(container_id, 6, "Fragmented MOV (.mov)");

                            // Color Space & Bit Depth
                            let colorspace_id = b"NukeVideoColorSpace\0";
                            builder.add_dropdown(ADBEBasicVideoGroup, colorspace_id, "Color Space & Bit Depth", 0);
                            builder.add_dropdown_item(colorspace_id, 0, "Rec.709 (SDR 8-bit)");
                            builder.add_dropdown_item(colorspace_id, 1, "Rec.2020 PQ (HDR 10-bit)");
                            builder.add_dropdown_item(colorspace_id, 2, "Rec.2020 HLG (HDR 10-bit)");

                            // 2. Audio Settings
                            builder.add_group(ADBETopParamGroup, ADBEAudioTabGroup, "Audio");
                            builder.add_group(ADBEAudioTabGroup, ADBEBasicAudioGroup, "Basic Audio Settings");
                            
                            builder.add_float_dropdown(ADBEBasicAudioGroup, ADBEAudioRatePerSecond, "Sample Rate", 48000.0); // 48000 Hz
                            builder.add_dropdown_item_float(ADBEAudioRatePerSecond, 32000.0, "32000 Hz");
                            builder.add_dropdown_item_float(ADBEAudioRatePerSecond, 44100.0, "44100 Hz");
                            builder.add_dropdown_item_float(ADBEAudioRatePerSecond, 48000.0, "48000 Hz");
                            builder.add_dropdown_item_float(ADBEAudioRatePerSecond, 88200.0, "88200 Hz");
                            builder.add_dropdown_item_float(ADBEAudioRatePerSecond, 96000.0, "96000 Hz");
                            
                            // 2 = kPrAudioChannelType_Stereo, 1 = Mono, 3 = 5.1
                            builder.add_dropdown(ADBEBasicAudioGroup, ADBEAudioNumChannels, "Channels", 2); 
                            builder.add_dropdown_item(ADBEAudioNumChannels, 1, "Mono");
                            builder.add_dropdown_item(ADBEAudioNumChannels, 2, "Stereo");
                            builder.add_dropdown_item(ADBEAudioNumChannels, 3, "5.1");
                            
                            // Audio Bitrate
                            builder.add_dropdown(ADBEBasicAudioGroup, ADBEAudioBitrate, "Audio Bitrate", 1); // default 128 kbps
                            builder.add_dropdown_item(ADBEAudioBitrate, 0, "96 kbps");
                            builder.add_dropdown_item(ADBEAudioBitrate, 1, "128 kbps");
                            builder.add_dropdown_item(ADBEAudioBitrate, 2, "160 kbps");
                            builder.add_dropdown_item(ADBEAudioBitrate, 3, "192 kbps");
                            builder.add_dropdown_item(ADBEAudioBitrate, 4, "256 kbps");
                            builder.add_dropdown_item(ADBEAudioBitrate, 5, "320 kbps");
                            
                            // 3. OBS-like Encoder Settings Group
                            let encoder_settings_group = b"NukeBitrateGroup\0";
                            builder.add_group(ADBEVideoTabGroup, encoder_settings_group, "Encoder Settings");
                            
                            // Rate Control
                            builder.add_dropdown(encoder_settings_group, ADBEVideoBitrateEncoding, "Rate Control", 0);
                            builder.add_dropdown_item(ADBEVideoBitrateEncoding, 0, "Constant Bitrate");
                            builder.add_dropdown_item(ADBEVideoBitrateEncoding, 1, "Constant QP");
                            builder.add_dropdown_item(ADBEVideoBitrateEncoding, 2, "Variable Bitrate");
                            builder.add_dropdown_item(ADBEVideoBitrateEncoding, 3, "Variable Bitrate with Target Quality");

                            builder.add_float_param(encoder_settings_group, ADBEVideoTargetBitrate, "Bitrate [Mbps]", 10.0, 0.1, 300.0);
                            builder.add_float_param(encoder_settings_group, ADBEVideoMaxBitrate, "Max Bitrate [Mbps]", 12.0, 0.1, 300.0);

                            // Presets
                            let nuke_preset_id = b"NukePreset\0";
                            builder.add_dropdown(encoder_settings_group, nuke_preset_id, "Preset", 4); // default P5
                            builder.add_dropdown_item(nuke_preset_id, 0, "P1: Fastest (Lowest Quality)");
                            builder.add_dropdown_item(nuke_preset_id, 1, "P2: Faster");
                            builder.add_dropdown_item(nuke_preset_id, 2, "P3: Fast");
                            builder.add_dropdown_item(nuke_preset_id, 3, "P4: Medium");
                            builder.add_dropdown_item(nuke_preset_id, 4, "P5: Slow (Good Quality)");
                            builder.add_dropdown_item(nuke_preset_id, 5, "P6: Slower");
                            builder.add_dropdown_item(nuke_preset_id, 6, "P7: Slowest (Best Quality)");

                            // Tuning
                            let nuke_tuning_id = b"NukeTuning\0";
                            builder.add_dropdown(encoder_settings_group, nuke_tuning_id, "Tuning", 0);
                            builder.add_dropdown_item(nuke_tuning_id, 0, "High Quality");
                            builder.add_dropdown_item(nuke_tuning_id, 1, "Low Latency");
                            builder.add_dropdown_item(nuke_tuning_id, 2, "Ultra Low Latency");

                            // Multipass Mode
                            let nuke_multipass_id = b"NukeMultipass\0";
                            builder.add_dropdown(encoder_settings_group, nuke_multipass_id, "Multipass Mode", 1); // default Quarter Resolution
                            builder.add_dropdown_item(nuke_multipass_id, 0, "Disabled");
                            builder.add_dropdown_item(nuke_multipass_id, 1, "Two Passes (Quarter Resolution)");
                            builder.add_dropdown_item(nuke_multipass_id, 2, "Two Passes (Full Resolution)");

                            // Look-ahead and AQ Checkboxes
                            let lookahead_id = b"NukeLookAhead\0";
                            builder.add_bool_param(encoder_settings_group, lookahead_id, "Look-ahead", true);
                            
                            let aq_id = b"NukeAdaptiveQuant\0";
                            builder.add_bool_param(encoder_settings_group, aq_id, "Adaptive Quantisation", true);

                            // B-Frames
                            let bframes_id = b"NukeBFrames\0";
                            builder.add_int_param(encoder_settings_group, bframes_id, "B-Frames", 2, 0, 4);

                            // B-Frame as Reference
                            let bframe_ref_id = b"NukeBFrameRef\0";
                            builder.add_dropdown(encoder_settings_group, bframe_ref_id, "B-Frame as Reference", 0);
                            builder.add_dropdown_item(bframe_ref_id, 0, "Disabled");
                            builder.add_dropdown_item(bframe_ref_id, 1, "Each B-Frame");
                            builder.add_dropdown_item(bframe_ref_id, 2, "Middle B-Frame");

                            // 4. Codec Specific Settings
                            let file_type = param_rec.fileType;
                            let nuka = u32::from_be_bytes(*b"NukA");
                            let nuk8 = u32::from_be_bytes(*b"Nuk8");
                            let nuk9 = u32::from_be_bytes(*b"Nuk9");
                            
                             if file_type == nuka {
                                let codec_group = b"NukeAV1EncodingGroup\0";
                                builder.add_group(ADBEVideoTabGroup, codec_group, "Encoding Settings");

                                builder.add_dropdown(codec_group, b"NukeAV1Encoder\0", "Performance", 0);
                                builder.add_dropdown_item(b"NukeAV1Encoder\0", 0, "Auto (Recommended)");
                                builder.add_dropdown_item(b"NukeAV1Encoder\0", 1, "Software Only (CPU)");
                                builder.add_dropdown_item(b"NukeAV1Encoder\0", 2, "Hardware Encoding (NVENC)");
                                builder.add_dropdown_item(b"NukeAV1Encoder\0", 3, "Hardware Encoding (AMF)");
                                builder.add_dropdown_item(b"NukeAV1Encoder\0", 4, "Hardware Encoding (QSV)");

                                builder.add_dropdown(codec_group, b"NukeAV1Profile\0", "Profile", 0);
                                builder.add_dropdown_item(b"NukeAV1Profile\0", 0, "Main");
                                builder.add_dropdown_item(b"NukeAV1Profile\0", 1, "High");
                                builder.add_dropdown_item(b"NukeAV1Profile\0", 2, "Professional");

                                builder.add_dropdown(codec_group, b"NukeAV1Level\0", "Level", 0);
                                builder.add_dropdown_item(b"NukeAV1Level\0", 0, "5.0");
                                builder.add_dropdown_item(b"NukeAV1Level\0", 1, "5.1");
                                builder.add_dropdown_item(b"NukeAV1Level\0", 2, "5.2");
                                builder.add_dropdown_item(b"NukeAV1Level\0", 3, "6.0");

                                builder.add_dropdown(codec_group, b"NukeAV1Tier\0", "Tier", 0);
                                builder.add_dropdown_item(b"NukeAV1Tier\0", 0, "Main");
                                builder.add_dropdown_item(b"NukeAV1Tier\0", 1, "High");

                                let adv_group = b"NukeAdvancedGroup\0";
                                builder.add_group(ADBEVideoTabGroup, adv_group, "Advanced Settings");
                                builder.add_float_param(adv_group, b"NukeAV1KeyFrame\0", "Key Frame Distance", 33.0, 1.0, 300.0);
                             } else if file_type == nuk8 {
                                let codec_group = b"NukeVP8EncodingGroup\0";
                                builder.add_group(ADBEVideoTabGroup, codec_group, "Encoding Settings");
                                
                                let adv_group = b"NukeAdvancedGroup\0";
                                builder.add_group(ADBEVideoTabGroup, adv_group, "Advanced Settings");
                                builder.add_float_param(adv_group, b"NukeVP8KeyFrame\0", "Key Frame Distance", 33.0, 1.0, 300.0);
                             } else if file_type == nuk9 {
                                let codec_group = b"NukeVP9EncodingGroup\0";
                                builder.add_group(ADBEVideoTabGroup, codec_group, "Encoding Settings");

                                builder.add_dropdown(codec_group, b"NukeVP9Profile\0", "Profile", 0);
                                builder.add_dropdown_item(b"NukeVP9Profile\0", 0, "Profile 0 (8-bit 4:2:0)");
                                builder.add_dropdown_item(b"NukeVP9Profile\0", 1, "Profile 1 (8-bit 4:2:2/4:4:4)");
                                builder.add_dropdown_item(b"NukeVP9Profile\0", 2, "Profile 2 (10/12-bit 4:2:0)");
                                builder.add_dropdown_item(b"NukeVP9Profile\0", 3, "Profile 3 (10/12-bit 4:2:2/4:4:4)");

                                let adv_group = b"NukeAdvancedGroup\0";
                                builder.add_group(ADBEVideoTabGroup, adv_group, "Advanced Settings");
                                builder.add_float_param(adv_group, b"NukeVP9KeyFrame\0", "Key Frame Distance", 33.0, 1.0, 300.0);
                             }
                            
                            // 4. Multiplexer Settings
                            let multiplexer_tab = b"ADBEExporterMultiplexerTab\0";
                            let multiplexer_group = b"ADBEExporterMultiplexerGroup\0";
                            let multiplexer_dropdown = b"ADBEExporterMultiplexerDropdown\0";

                            builder.add_group(ADBETopParamGroup, multiplexer_tab, "Multiplexer");
                            builder.add_group(multiplexer_tab, multiplexer_group, "Basic Settings");
                            
                            builder.add_dropdown(multiplexer_group, multiplexer_dropdown, "Multiplexer", 0);
                            builder.add_dropdown_item(multiplexer_dropdown, 0, "MP4");
                            builder.add_dropdown_item(multiplexer_dropdown, 1, "3GPP");
                            builder.add_dropdown_item(multiplexer_dropdown, 2, "None");

                            builder.set_params_version(1);
                        }
                        
                        if let Some(release_suite) = basic_suite.ReleaseSuite {
                            release_suite(kPrSDKExportParamSuite.as_ptr() as *const i8, kPrSDKExportParamSuiteVersion as i32);
                        }
                    }
                }
            }
        }
    }
    malNoError as prMALError
}

pub unsafe fn handle_post_process_params(std_parms: *mut exportStdParms, param1: *mut c_void) -> prMALError {
    use std::os::raw::c_char;
    let rec = param1 as *mut exPostProcessParamsRec;
    if !rec.is_null() && !std_parms.is_null() {
        let rec = &mut *rec;
        let std_parms = &*std_parms;
        
        if let Some(get_suite) = std_parms.getSPBasicSuite {
            let basic_suite = get_suite();
            if !basic_suite.is_null() {
                let basic_suite = &*basic_suite;
                if let Some(acquire_suite) = basic_suite.AcquireSuite {
                    let mut param_suite_ptr: *const core::ffi::c_void = core::ptr::null();
                    acquire_suite(
                        kPrSDKExportParamSuite.as_ptr() as *const i8,
                        kPrSDKExportParamSuiteVersion as i32,
                        &mut param_suite_ptr,
                    );
                    
                    if !param_suite_ptr.is_null() {
                        let param_suite = &*(param_suite_ptr as *const PrSDKExportParamSuite);
                        
                        let set_name = |param_id: &[u8], name: &str| {
                            if let Some(set_param_name) = param_suite.SetParamName {
                                set_param_name(
                                    rec.exporterPluginID,
                                    0, // multiGroupIndex
                                    param_id.as_ptr() as *const c_char,
                                    crate::leak_utf16(name)
                                );
                            }
                        };
                        
                        // Set names for groups and parameters
                        set_name(ADBETopParamGroup, "Video");
                        set_name(ADBEVideoTabGroup, "Video");
                        set_name(ADBEBasicVideoGroup, "Basic Video Settings");
                        set_name(ADBEVideoWidth, "Frame Width");
                        set_name(ADBEVideoHeight, "Frame Height");
                        set_name(ADBEVideoFPS, "Frame Rate");
                        set_name(ADBEVideoAspect, "Pixel Aspect Ratio");
                        set_name(ADBEVideoFieldType, "Field Order");
                        set_name(b"ADBEVideoContainer\0", "File Extension");
                        set_name(b"NukeVideoColorSpace\0", "Color Space & Bit Depth");
                        
                        set_name(ADBEAudioTabGroup, "Audio");
                        set_name(ADBEBasicAudioGroup, "Basic Audio Settings");
                        set_name(ADBEAudioRatePerSecond, "Sample Rate");
                        set_name(ADBEAudioNumChannels, "Channels");
                        set_name(ADBEAudioBitrate, "Audio Bitrate");
                        
                        set_name(b"NukeBitrateGroup\0", "Encoder Settings");
                        set_name(ADBEVideoBitrateEncoding, "Rate Control");
                        
                        set_name(b"ADBEExporterMultiplexerTab\0", "Multiplexer");
                        set_name(b"ADBEExporterMultiplexerGroup\0", "Basic Settings");
                        set_name(b"ADBEExporterMultiplexerDropdown\0", "Multiplexer");

                        // Get current Rate Control choice to dynamically rename bitrate parameters
                        let mut rc_val: exParamValues = core::mem::zeroed();
                        let mut target_label = "Bitrate [Mbps]";
                        let mut max_label = "Max Bitrate [Mbps]";

                        if let Some(get_param_value) = param_suite.GetParamValue {
                            get_param_value(rec.exporterPluginID, 0, ADBEVideoBitrateEncoding.as_ptr() as *const c_char, &mut rc_val);
                            let rate_control = rc_val.value.__bindgen_anon_1.intValue;
                            match rate_control {
                                0 => { // CBR
                                    target_label = "Target Bitrate [Mbps]";
                                    max_label = "Max Bitrate (Unused in CBR)";
                                }
                                1 => { // CQP
                                    target_label = "Constant QP (CQ / CQP Value)";
                                    max_label = "Max Bitrate (Unused in CQP)";
                                }
                                2 => { // VBR
                                    target_label = "Target Bitrate [Mbps]";
                                    max_label = "Max Bitrate [Mbps]";
                                }
                                3 => { // VBR with Target Quality
                                    target_label = "Target Quality (CQ Value)";
                                    max_label = "Max Bitrate [Mbps]";
                                }
                                _ => {}
                            }
                        }

                        set_name(ADBEVideoTargetBitrate, target_label);
                        set_name(ADBEVideoMaxBitrate, max_label);
                        set_name(b"NukePreset\0", "Preset");
                        set_name(b"NukeTuning\0", "Tuning");
                        set_name(b"NukeMultipass\0", "Multipass Mode");
                        set_name(b"NukeLookAhead\0", "Look-ahead");
                        set_name(b"NukeAdaptiveQuant\0", "Adaptive Quantisation");
                        set_name(b"NukeBFrames\0", "B-Frames");
                        set_name(b"NukeBFrameRef\0", "B-Frame as Reference");
                        
                        let file_type = rec.fileType;
                        let nuka = u32::from_be_bytes(*b"NukA");
                        let nuk8 = u32::from_be_bytes(*b"Nuk8");
                        let nuk9 = u32::from_be_bytes(*b"Nuk9");
                        
                        if file_type == nuka {
                            set_name(b"NukeAV1EncodingGroup\0", "Encoding Settings");
                            set_name(b"NukeAV1Encoder\0", "Performance");
                            set_name(b"NukeAV1Profile\0", "Profile");
                            set_name(b"NukeAV1Level\0", "Level");
                            set_name(b"NukeAV1Tier\0", "Tier");
                            
                            set_name(b"NukeAdvancedGroup\0", "Advanced Settings");
                            set_name(b"NukeAV1KeyFrame\0", "Key Frame Distance");
                        } else if file_type == nuk8 {
                            set_name(b"NukeVP8EncodingGroup\0", "Encoding Settings");
                            set_name(b"NukeAdvancedGroup\0", "Advanced Settings");
                            set_name(b"NukeVP8KeyFrame\0", "Key Frame Distance");
                        } else if file_type == nuk9 {
                            set_name(b"NukeVP9EncodingGroup\0", "Encoding Settings");
                            set_name(b"NukeVP9Profile\0", "Profile");
                            set_name(b"NukeAdvancedGroup\0", "Advanced Settings");
                            set_name(b"NukeVP9KeyFrame\0", "Key Frame Distance");
                        }
                        
                        rebuild_dropdowns(rec.exporterPluginID, file_type, param_suite);
                        
                        if let Some(release_suite) = basic_suite.ReleaseSuite {
                            release_suite(kPrSDKExportParamSuite.as_ptr() as *const i8, kPrSDKExportParamSuiteVersion as i32);
                        }
                    }
                }
            }
        }
    }
    malNoError as prMALError
}

unsafe fn rebuild_dropdowns(ex_id: csSDK_uint32, file_type: csSDK_uint32, param_suite: &PrSDKExportParamSuite) {
    use std::os::raw::c_char;

    let clear_constrained = |param_id: &[u8]| {
        if let Some(clear_values) = param_suite.ClearConstrainedValues {
            clear_values(ex_id, 0, param_id.as_ptr() as *const c_char);
        }
    };

    let add_val = |param_id: &[u8], value: i32, name: &str| {
        if let Some(add_pair) = param_suite.AddConstrainedValuePair {
            let mut val_rec: exOneParamValueRec = core::mem::zeroed();
            val_rec.__bindgen_anon_1.intValue = value;
            add_pair(
                ex_id,
                0,
                param_id.as_ptr() as *const c_char,
                &val_rec,
                crate::leak_utf16(name),
            );
        }
    };

    let add_val_float = |param_id: &[u8], value: f64, name: &str| {
        if let Some(add_pair) = param_suite.AddConstrainedValuePair {
            let mut val_rec: exOneParamValueRec = core::mem::zeroed();
            val_rec.__bindgen_anon_1.floatValue = value;
            add_pair(
                ex_id,
                0,
                param_id.as_ptr() as *const c_char,
                &val_rec,
                crate::leak_utf16(name),
            );
        }
    };

    let add_val_time = |param_id: &[u8], value: i64, name: &str| {
        if let Some(add_pair) = param_suite.AddConstrainedValuePair {
            let mut val_rec: exOneParamValueRec = core::mem::zeroed();
            val_rec.__bindgen_anon_1.timeValue = value;
            add_pair(
                ex_id,
                0,
                param_id.as_ptr() as *const c_char,
                &val_rec,
                crate::leak_utf16(name),
            );
        }
    };

    let add_val_ratio = |param_id: &[u8], num: i32, den: i32, name: &str| {
        if let Some(add_pair) = param_suite.AddConstrainedValuePair {
            let mut val_rec: exOneParamValueRec = core::mem::zeroed();
            val_rec.__bindgen_anon_1.ratioValue.numerator = num;
            val_rec.__bindgen_anon_1.ratioValue.denominator = den;
            add_pair(
                ex_id,
                0,
                param_id.as_ptr() as *const c_char,
                &val_rec,
                crate::leak_utf16(name),
            );
        }
    };

    // Width
    clear_constrained(ADBEVideoWidth);
    add_val(ADBEVideoWidth, 4096, "4K (4096)");
    add_val(ADBEVideoWidth, 3840, "UHD (3840)");
    add_val(ADBEVideoWidth, 2560, "Quad HD (2560)");
    add_val(ADBEVideoWidth, 1920, "Full HD (1920)");
    add_val(ADBEVideoWidth, 1280, "HD (1280)");
    add_val(ADBEVideoWidth, 854, "SD NTSC Wide (854)");
    add_val(ADBEVideoWidth, 720, "SD NTSC (720)");
    add_val(ADBEVideoWidth, -1, "Custom");

    // Height
    clear_constrained(ADBEVideoHeight);
    add_val(ADBEVideoHeight, 2160, "4K / UHD (2160)");
    add_val(ADBEVideoHeight, 1440, "Quad HD (1440)");
    add_val(ADBEVideoHeight, 1080, "Full HD (1080)");
    add_val(ADBEVideoHeight, 720, "HD (720)");
    add_val(ADBEVideoHeight, 480, "SD NTSC (480)");
    add_val(ADBEVideoHeight, -1, "Custom");

    // Frame Rate
    clear_constrained(ADBEVideoFPS);
    add_val_time(ADBEVideoFPS, 10594627200, "23.976");
    add_val_time(ADBEVideoFPS, 10584000000, "24");
    add_val_time(ADBEVideoFPS, 10160640000, "25");
    add_val_time(ADBEVideoFPS, 8475283200, "29.97");
    add_val_time(ADBEVideoFPS, 8467200000, "30");
    add_val_time(ADBEVideoFPS, 5080320000, "50");
    add_val_time(ADBEVideoFPS, 4237641600, "59.94");
    add_val_time(ADBEVideoFPS, 4233600000, "60");
    add_val_time(ADBEVideoFPS, 2822400000, "90");
    add_val_time(ADBEVideoFPS, 2540160000, "100");
    add_val_time(ADBEVideoFPS, 2118820800, "119.88");
    add_val_time(ADBEVideoFPS, 2116800000, "120");
    add_val_time(ADBEVideoFPS, 1764000000, "144");
    add_val_time(ADBEVideoFPS, 1058400000, "240");
    add_val_time(ADBEVideoFPS, 846720000, "300");
    add_val_time(ADBEVideoFPS, -1, "Custom");

    // Aspect
    clear_constrained(ADBEVideoAspect);
    add_val_ratio(ADBEVideoAspect, 1, 1, "Square Pixels (1.0)");
    add_val_ratio(ADBEVideoAspect, 10, 11, "D1/DV NTSC (0.9091)");
    add_val_ratio(ADBEVideoAspect, 40, 33, "D1/DV NTSC Widescreen 16:9 (1.2121)");
    add_val_ratio(ADBEVideoAspect, 59, 54, "D1/DV PAL (1.0940)");
    add_val_ratio(ADBEVideoAspect, 118, 81, "D1/DV PAL Widescreen 16:9 (1.4587)");
    add_val_ratio(ADBEVideoAspect, 2, 1, "Anamorphic 2:1 (2.0)");
    add_val_ratio(ADBEVideoAspect, 4, 3, "HD Anamorphic 1080 (1.333)");
    add_val_ratio(ADBEVideoAspect, 3, 2, "DVCPRO HD (1.5)");
    add_val_ratio(ADBEVideoAspect, -1, -1, "Custom");

    // Field Type
    clear_constrained(ADBEVideoFieldType);
    add_val(ADBEVideoFieldType, 0, "Progressive");

    // Container File Extension
    let container_id = b"ADBEVideoContainer\0";
    clear_constrained(container_id);
    add_val(container_id, 0, "Matroska Video (.mkv)");
    add_val(container_id, 1, "MPEG-4 (.mp4)");
    add_val(container_id, 2, "Hybrid MP4 (.mp4)");
    add_val(container_id, 3, "Fragmented MP4 (.mp4)");
    add_val(container_id, 4, "QuickTime (.mov)");
    add_val(container_id, 5, "Hybrid MOV (.mov)");
    add_val(container_id, 6, "Fragmented MOV (.mov)");

    // Color Space
    let colorspace_id = b"NukeVideoColorSpace\0";
    clear_constrained(colorspace_id);
    add_val(colorspace_id, 0, "Rec.709 (SDR 8-bit)");
    add_val(colorspace_id, 1, "Rec.2020 PQ (HDR 10-bit)");
    add_val(colorspace_id, 2, "Rec.2020 HLG (HDR 10-bit)");

    // Audio Rate Per Second
    clear_constrained(ADBEAudioRatePerSecond);
    add_val_float(ADBEAudioRatePerSecond, 32000.0, "32000 Hz");
    add_val_float(ADBEAudioRatePerSecond, 44100.0, "44100 Hz");
    add_val_float(ADBEAudioRatePerSecond, 48000.0, "48000 Hz");
    add_val_float(ADBEAudioRatePerSecond, 88200.0, "88200 Hz");
    add_val_float(ADBEAudioRatePerSecond, 96000.0, "96000 Hz");

    // Audio Channels
    clear_constrained(ADBEAudioNumChannels);
    add_val(ADBEAudioNumChannels, 1, "Mono");
    add_val(ADBEAudioNumChannels, 2, "Stereo");
    add_val(ADBEAudioNumChannels, 3, "5.1");

    // Audio Bitrate
    clear_constrained(ADBEAudioBitrate);
    add_val(ADBEAudioBitrate, 0, "96 kbps");
    add_val(ADBEAudioBitrate, 1, "128 kbps");
    add_val(ADBEAudioBitrate, 2, "160 kbps");
    add_val(ADBEAudioBitrate, 3, "192 kbps");
    add_val(ADBEAudioBitrate, 4, "256 kbps");
    add_val(ADBEAudioBitrate, 5, "320 kbps");

    // Rate Control
    clear_constrained(ADBEVideoBitrateEncoding);
    add_val(ADBEVideoBitrateEncoding, 0, "Constant Bitrate");
    add_val(ADBEVideoBitrateEncoding, 1, "Constant QP");
    add_val(ADBEVideoBitrateEncoding, 2, "Variable Bitrate");
    add_val(ADBEVideoBitrateEncoding, 3, "Variable Bitrate with Target Quality");

    // Presets
    let nuke_preset_id = b"NukePreset\0";
    clear_constrained(nuke_preset_id);
    add_val(nuke_preset_id, 0, "P1: Fastest (Lowest Quality)");
    add_val(nuke_preset_id, 1, "P2: Faster");
    add_val(nuke_preset_id, 2, "P3: Fast");
    add_val(nuke_preset_id, 3, "P4: Medium");
    add_val(nuke_preset_id, 4, "P5: Slow (Good Quality)");
    add_val(nuke_preset_id, 5, "P6: Slower");
    add_val(nuke_preset_id, 6, "P7: Slowest (Best Quality)");

    // Tuning
    let nuke_tuning_id = b"NukeTuning\0";
    clear_constrained(nuke_tuning_id);
    add_val(nuke_tuning_id, 0, "High Quality");
    add_val(nuke_tuning_id, 1, "Low Latency");
    add_val(nuke_tuning_id, 2, "Ultra Low Latency");

    // Multipass Mode
    let nuke_multipass_id = b"NukeMultipass\0";
    clear_constrained(nuke_multipass_id);
    add_val(nuke_multipass_id, 0, "Disabled");
    add_val(nuke_multipass_id, 1, "Two Passes (Quarter Resolution)");
    add_val(nuke_multipass_id, 2, "Two Passes (Full Resolution)");

    // B-Frame Ref
    let bframe_ref_id = b"NukeBFrameRef\0";
    clear_constrained(bframe_ref_id);
    add_val(bframe_ref_id, 0, "Disabled");
    add_val(bframe_ref_id, 1, "Each B-Frame");
    add_val(bframe_ref_id, 2, "Middle B-Frame");

    // Exporter Multiplexer
    let multiplexer_dropdown = b"ADBEExporterMultiplexerDropdown\0";
    clear_constrained(multiplexer_dropdown);
    add_val(multiplexer_dropdown, 0, "MP4");
    add_val(multiplexer_dropdown, 1, "3GPP");
    add_val(multiplexer_dropdown, 2, "None");

    let nuka = u32::from_be_bytes(*b"NukA");
    let nuk9 = u32::from_be_bytes(*b"Nuk9");

    if file_type == nuka {
        let encoder_id = b"NukeAV1Encoder\0";
        clear_constrained(encoder_id);
        add_val(encoder_id, 0, "Auto (Recommended)");
        add_val(encoder_id, 1, "Software Only (CPU)");
        add_val(encoder_id, 2, "Hardware Encoding (NVENC)");
        add_val(encoder_id, 3, "Hardware Encoding (AMF)");
        add_val(encoder_id, 4, "Hardware Encoding (QSV)");

        let profile_id = b"NukeAV1Profile\0";
        clear_constrained(profile_id);
        add_val(profile_id, 0, "Main");
        add_val(profile_id, 1, "High");
        add_val(profile_id, 2, "Professional");

        let level_id = b"NukeAV1Level\0";
        clear_constrained(level_id);
        add_val(level_id, 0, "5.0");
        add_val(level_id, 1, "5.1");
        add_val(level_id, 2, "5.2");
        add_val(level_id, 3, "6.0");

        let tier_id = b"NukeAV1Tier\0";
        clear_constrained(tier_id);
        add_val(tier_id, 0, "Main");
        add_val(tier_id, 1, "High");
    } else if file_type == nuk9 {
        let profile_id = b"NukeVP9Profile\0";
        clear_constrained(profile_id);
        add_val(profile_id, 0, "Profile 0 (8-bit 4:2:0)");
        add_val(profile_id, 1, "Profile 1 (8-bit 4:2:2/4:4:4)");
        add_val(profile_id, 2, "Profile 2 (10/12-bit 4:2:0)");
        add_val(profile_id, 3, "Profile 3 (10/12-bit 4:2:2/4:4:4)");
    }
}

