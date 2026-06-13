use crate::ffmpeg_ffi::*;
use std::ffi::CString;
use std::ptr;

pub struct FFmpegEncoder {
    fmt_ctx: *mut AVFormatContext,
    codec_ctx: *mut AVCodecContext,
    stream: *mut AVStream,
    sws_ctx: *mut SwsContext,
    frame: *mut AVFrame,
    pkt: *mut AVPacket,
    video_pts: i64,
    colorspace: i32,
    temp_bgra64: Vec<u16>,

    // Audio fields
    audio_codec_ctx: *mut AVCodecContext,
    audio_stream: *mut AVStream,
    swr_ctx: *mut SwrContext,
    audio_frame: *mut AVFrame,
    audio_pts: i64,
    audio_channels: i32,
    audio_in_sample_rate: i32,
    audio_buffer: Vec<Vec<f32>>,
    has_audio: bool,
}

impl FFmpegEncoder {
    pub unsafe fn new(
        output_path: &str,
        vcodec_name: &str,
        width: i32,
        height: i32,
        fps: i32,
        bitrate: i64,
        audio_enabled: bool,
        audio_codec_name: &str,
        sample_rate: i32,
        channels: i32,
        bitrate_mode: i32, // 0 = CBR, 1 = CQP, 2 = VBR, 3 = VBR (Target Quality)
        max_bitrate: i64,
        audio_bitrate: i32,
        gop_size: i32,
        profile: i32,
        level: i32,
        tier: i32,
        colorspace: i32,
        
        // OBS-like options
        preset: i32,
        tuning: i32,
        multipass: i32,
        lookahead: bool,
        spatial_aq: bool,
        bframes: i32,
        bframe_ref: i32,
        container_choice: i32,
    ) -> Result<Self, String> {
        let c_output_path = CString::new(output_path).map_err(|_| "Invalid path")?;
        let c_vcodec_name = CString::new(vcodec_name).map_err(|_| "Invalid codec name")?;

        let mut fmt_ctx: *mut AVFormatContext = ptr::null_mut();
        if avformat_alloc_output_context2(&mut fmt_ctx, ptr::null_mut(), ptr::null(), c_output_path.as_ptr()) < 0 {
            return Err("Failed to allocate output context".into());
        }

        // --- Video Stream Setup ---
        let codec = avcodec_find_encoder_by_name(c_vcodec_name.as_ptr());
        if codec.is_null() {
            avformat_free_context(fmt_ctx);
            return Err(format!("Codec '{}' not found", vcodec_name));
        }

        let stream = avformat_new_stream(fmt_ctx, codec);
        if stream.is_null() {
            avformat_free_context(fmt_ctx);
            return Err("Failed to create video stream".into());
        }
        (*stream).id = ((*fmt_ctx).nb_streams - 1) as i32;

        let mut codec_ctx = avcodec_alloc_context3(codec);
        if codec_ctx.is_null() {
            avformat_free_context(fmt_ctx);
            return Err("Failed to allocate video codec context".into());
        }

        (*codec_ctx).width = width;
        (*codec_ctx).height = height;
        (*codec_ctx).time_base = AVRational { num: 1, den: fps };
        (*codec_ctx).framerate = AVRational { num: fps, den: 1 };
        
        let dst_fmt = if colorspace == 1 || colorspace == 2 {
            AV_PIX_FMT_YUV420P10LE
        } else {
            AV_PIX_FMT_YUV420P
        };
        (*codec_ctx).pix_fmt = dst_fmt;
        (*codec_ctx).bit_rate = bitrate;

        if colorspace == 1 { // Rec.2020 PQ
            (*codec_ctx).color_range = AVCOL_RANGE_MPEG;
            (*codec_ctx).color_primaries = AVCOL_PRI_BT2020;
            (*codec_ctx).color_trc = AVCOL_TRC_SMPTE2084;
            (*codec_ctx).colorspace = AVCOL_SPC_BT2020_NCL;
        } else if colorspace == 2 { // Rec.2020 HLG
            (*codec_ctx).color_range = AVCOL_RANGE_MPEG;
            (*codec_ctx).color_primaries = AVCOL_PRI_BT2020;
            (*codec_ctx).color_trc = AVCOL_TRC_ARIB_STD_B67;
            (*codec_ctx).colorspace = AVCOL_SPC_BT2020_NCL;
        } else { // SDR BT.709
            (*codec_ctx).color_range = AVCOL_RANGE_MPEG;
            (*codec_ctx).color_primaries = AVCOL_PRI_BT709;
            (*codec_ctx).color_trc = AVCOL_TRC_BT709;
            (*codec_ctx).colorspace = AVCOL_SPC_BT709;
        }

        // GOP / Keyframe distance (UI default is 33.0, representing frames, not seconds. If value is very small like <= 5, it might be treated as seconds)
        if gop_size > 0 {
            if gop_size <= 5 {
                (*codec_ctx).gop_size = gop_size * fps;
            } else {
                (*codec_ctx).gop_size = gop_size;
            }
        } else {
            (*codec_ctx).gop_size = 250; // auto fallback
        }

        // B-Frames
        (*codec_ctx).max_b_frames = bframes;

        // Apply rate control to codec context
        if vcodec_name.contains("nvenc") {
            let rc_opt = CString::new("rc").unwrap();
            let rc_val = match bitrate_mode {
                0 => CString::new("cbr").unwrap(),
                1 => CString::new("constqp").unwrap(),
                2 => CString::new("vbr").unwrap(),
                3 => CString::new("vbr").unwrap(),
                _ => CString::new("cbr").unwrap(),
            };
            av_opt_set((*codec_ctx).priv_data, rc_opt.as_ptr(), rc_val.as_ptr(), 0);

            if bitrate_mode == 0 { // CBR
                (*codec_ctx).bit_rate = bitrate;
                (*codec_ctx).rc_max_rate = bitrate;
                (*codec_ctx).rc_buffer_size = (bitrate * 2) as i32;
            } else if bitrate_mode == 1 { // CQP
                let qp_val = CString::new("20").unwrap(); // default QP
                av_opt_set((*codec_ctx).priv_data, CString::new("qp").unwrap().as_ptr(), qp_val.as_ptr(), 0);
            } else if bitrate_mode == 2 { // VBR
                (*codec_ctx).bit_rate = bitrate;
                (*codec_ctx).rc_max_rate = max_bitrate;
                (*codec_ctx).rc_buffer_size = (max_bitrate * 2) as i32;
            } else if bitrate_mode == 3 { // VBR with Target Quality
                (*codec_ctx).bit_rate = bitrate;
                let cq_val = CString::new("20").unwrap();
                av_opt_set((*codec_ctx).priv_data, CString::new("cq").unwrap().as_ptr(), cq_val.as_ptr(), 0);
            }

            // Multipass Mode
            let mp_opt = CString::new("multipass").unwrap();
            let mp_val = match multipass {
                0 => CString::new("disabled").unwrap(),
                1 => CString::new("qres").unwrap(),
                2 => CString::new("fullres").unwrap(),
                _ => CString::new("qres").unwrap(),
            };
            av_opt_set((*codec_ctx).priv_data, mp_opt.as_ptr(), mp_val.as_ptr(), 0);

            // Look-ahead
            let la_val = if lookahead { "1" } else { "0" };
            av_opt_set((*codec_ctx).priv_data, CString::new("rc-lookahead").unwrap().as_ptr(), CString::new(la_val).unwrap().as_ptr(), 0);

            // Adaptive Quantisation (Spatial AQ)
            let aq_val = if spatial_aq { "1" } else { "0" };
            av_opt_set((*codec_ctx).priv_data, CString::new("spatial-aq").unwrap().as_ptr(), CString::new(aq_val).unwrap().as_ptr(), 0);

            // B-Frame as Reference
            let b_ref_val = match bframe_ref {
                0 => CString::new("disabled").unwrap(),
                1 => CString::new("each").unwrap(),
                2 => CString::new("middle").unwrap(),
                _ => CString::new("disabled").unwrap(),
            };
            av_opt_set((*codec_ctx).priv_data, CString::new("b_ref_mode").unwrap().as_ptr(), b_ref_val.as_ptr(), 0);

            // Configure Presets & Tuning for NVENC
            let preset_str = match preset {
                0 => Some("p1"),
                1 => Some("p2"),
                2 => Some("p3"),
                3 => Some("p4"),
                4 => Some("p5"),
                5 => Some("p6"),
                6 => Some("p7"),
                _ => Some("p4"),
            };
            if let Some(p_str) = preset_str {
                av_opt_set((*codec_ctx).priv_data, CString::new("preset").unwrap().as_ptr(), CString::new(p_str).unwrap().as_ptr(), 0);
            }

            let tune_str = match tuning {
                0 => Some("hq"),
                1 => Some("ll"),
                2 => Some("ull"),
                _ => Some("hq"),
            };
            if let Some(t_str) = tune_str {
                av_opt_set((*codec_ctx).priv_data, CString::new("tune").unwrap().as_ptr(), CString::new(t_str).unwrap().as_ptr(), 0);
            }
        } else {
            // Software (libaom / libvpx) / QSV / AMF fallbacks
            if bitrate_mode == 0 || bitrate_mode == 2 { // CBR or VBR
                (*codec_ctx).bit_rate = bitrate;
                if bitrate_mode == 0 {
                    (*codec_ctx).rc_max_rate = bitrate;
                } else {
                    (*codec_ctx).rc_max_rate = max_bitrate;
                }
            } else { // Constant Quality / QP
                let crf_val = if vcodec_name.contains("av1") { "26" } else { "28" };
                av_opt_set((*codec_ctx).priv_data, CString::new("crf").unwrap().as_ptr(), CString::new(crf_val).unwrap().as_ptr(), 0);
            }

            // Map CPU speed presets
            let cpu_used_val = match preset {
                0 => Some("8"), // fastest
                1 => Some("7"),
                2 => Some("6"),
                3 => Some("5"),
                4 => Some("4"), // normal
                5 => Some("3"),
                6 => Some("2"), // slowest / best quality
                _ => Some("4"),
            };
            if let Some(cpu_val) = cpu_used_val {
                if vcodec_name.contains("libaom") || vcodec_name.contains("libvpx") {
                    av_opt_set((*codec_ctx).priv_data, CString::new("cpu-used").unwrap().as_ptr(), CString::new(cpu_val).unwrap().as_ptr(), 0);
                }
            }
        }

        // Configure profile if it's not default (0 or greater)
        if profile >= 0 {
            let profile_str = if vcodec_name.contains("av1") {
                match profile {
                    0 => Some("main"),
                    1 => Some("high"),
                    2 => Some("professional"),
                    _ => None,
                }
            } else if vcodec_name.contains("vp9") {
                match profile {
                    0 => Some("0"),
                    1 => Some("1"),
                    2 => Some("2"),
                    3 => Some("3"),
                    _ => None,
                }
            } else {
                None
            };
            
            if let Some(p_str) = profile_str {
                let p_opt = CString::new("profile").unwrap();
                let p_val = CString::new(p_str).unwrap();
                av_opt_set((*codec_ctx).priv_data, p_opt.as_ptr(), p_val.as_ptr(), 0);
            }
        }

        // Configure level
        if level >= 0 && vcodec_name.contains("av1") {
            let level_str = match level {
                0 => Some("5.0"),
                1 => Some("5.1"),
                2 => Some("5.2"),
                3 => Some("6.0"),
                _ => None,
            };
            if let Some(l_str) = level_str {
                let l_opt = CString::new("level").unwrap();
                let l_val = CString::new(l_str).unwrap();
                av_opt_set((*codec_ctx).priv_data, l_opt.as_ptr(), l_val.as_ptr(), 0);
            }
        }

        // Configure tier
        if tier >= 0 && vcodec_name.contains("av1") {
            let tier_str = match tier {
                0 => Some("main"),
                1 => Some("high"),
                _ => None,
            };
            if let Some(t_str) = tier_str {
                let t_opt = CString::new("tier").unwrap();
                let t_val = CString::new(t_str).unwrap();
                av_opt_set((*codec_ctx).priv_data, t_opt.as_ptr(), t_val.as_ptr(), 0);
            }
        }

        (*codec_ctx).time_base = (*stream).time_base;

        if ((*(*fmt_ctx).oformat).flags as u32 & AVFMT_GLOBALHEADER) != 0 {
            (*codec_ctx).flags |= AV_CODEC_FLAG_GLOBAL_HEADER as i32;
        }

        (*codec_ctx).thread_count = 0;

        let err = avcodec_open2(codec_ctx, codec, ptr::null_mut());
        if err < 0 {
            let mut errbuf = [0i8; 256];
            av_strerror(err, errbuf.as_mut_ptr(), errbuf.len());
            let err_str = unsafe { std::ffi::CStr::from_ptr(errbuf.as_ptr()) }.to_string_lossy().into_owned();
            avcodec_free_context(&mut codec_ctx as *mut _);
            avformat_free_context(fmt_ctx);
            return Err(format!("Failed to open video codec: {} ({})", err_str, err));
        }

        if avcodec_parameters_from_context((*stream).codecpar, codec_ctx) < 0 {
            avcodec_free_context(&mut codec_ctx as *mut _);
            avformat_free_context(fmt_ctx);
            return Err("Failed to copy video codec parameters".into());
        }

        // --- Audio Stream Setup ---
        let mut audio_codec_ctx: *mut AVCodecContext = ptr::null_mut();
        let mut audio_stream: *mut AVStream = ptr::null_mut();
        let mut swr_ctx: *mut SwrContext = ptr::null_mut();
        let mut audio_frame: *mut AVFrame = ptr::null_mut();
        let mut audio_buffer = Vec::new();

        if audio_enabled {
            let c_audio_codec_name = CString::new(audio_codec_name).map_err(|_| "Invalid audio codec name")?;
            let audio_codec = avcodec_find_encoder_by_name(c_audio_codec_name.as_ptr());
            if audio_codec.is_null() {
                avcodec_free_context(&mut codec_ctx as *mut _);
                avformat_free_context(fmt_ctx);
                return Err(format!("Audio codec '{}' not found", audio_codec_name));
            }

            audio_stream = avformat_new_stream(fmt_ctx, audio_codec);
            if audio_stream.is_null() {
                avcodec_free_context(&mut codec_ctx as *mut _);
                avformat_free_context(fmt_ctx);
                return Err("Failed to create audio stream".into());
            }
            (*audio_stream).id = ((*fmt_ctx).nb_streams - 1) as i32;

            audio_codec_ctx = avcodec_alloc_context3(audio_codec);
            if audio_codec_ctx.is_null() {
                avcodec_free_context(&mut codec_ctx as *mut _);
                avformat_free_context(fmt_ctx);
                return Err("Failed to allocate audio codec context".into());
            }

            // Select best supported sample format and sample rate
            let mut sample_fmt = AV_SAMPLE_FMT_FLTP;
            if !(*audio_codec).sample_fmts.is_null() {
                let mut i = 0;
                let mut found_fltp = false;
                while *(*audio_codec).sample_fmts.add(i) != AV_SAMPLE_FMT_NONE {
                    let fmt = *(*audio_codec).sample_fmts.add(i);
                    if fmt == AV_SAMPLE_FMT_FLTP {
                        sample_fmt = fmt;
                        found_fltp = true;
                        break;
                    }
                    i += 1;
                }
                if !found_fltp {
                    sample_fmt = *(*audio_codec).sample_fmts;
                }
            }

            let mut target_sample_rate = sample_rate;
            if !(*audio_codec).supported_samplerates.is_null() {
                let mut i = 0;
                let mut best_rate = 0;
                while *(*audio_codec).supported_samplerates.add(i) != 0 {
                    let rate = *(*audio_codec).supported_samplerates.add(i);
                    if rate == sample_rate {
                        best_rate = rate;
                        break;
                    }
                    if best_rate == 0 || rate == 48000 {
                        best_rate = rate;
                    }
                    i += 1;
                }
                if best_rate != 0 {
                    target_sample_rate = best_rate;
                }
            }

            (*audio_codec_ctx).sample_fmt = sample_fmt;
            (*audio_codec_ctx).sample_rate = target_sample_rate;
            (*audio_codec_ctx).time_base = AVRational { num: 1, den: target_sample_rate };

            av_channel_layout_default(&mut (*audio_codec_ctx).ch_layout, channels);
            (*audio_codec_ctx).bit_rate = audio_bitrate as i64;

            if ((*(*fmt_ctx).oformat).flags as u32 & AVFMT_GLOBALHEADER) != 0 {
                (*audio_codec_ctx).flags |= AV_CODEC_FLAG_GLOBAL_HEADER as i32;
            }

            let err = avcodec_open2(audio_codec_ctx, audio_codec, ptr::null_mut());
            if err < 0 {
                let mut errbuf = [0i8; 256];
                av_strerror(err, errbuf.as_mut_ptr(), errbuf.len());
                let err_str = unsafe { std::ffi::CStr::from_ptr(errbuf.as_ptr()) }.to_string_lossy().into_owned();
                avcodec_free_context(&mut audio_codec_ctx as *mut _);
                avcodec_free_context(&mut codec_ctx as *mut _);
                avformat_free_context(fmt_ctx);
                return Err(format!("Failed to open audio codec: {} ({})", err_str, err));
            }

            if avcodec_parameters_from_context((*audio_stream).codecpar, audio_codec_ctx) < 0 {
                avcodec_free_context(&mut audio_codec_ctx as *mut _);
                avcodec_free_context(&mut codec_ctx as *mut _);
                avformat_free_context(fmt_ctx);
                return Err("Failed to copy audio codec parameters".into());
            }

            // Resampler setup
            let mut in_ch_layout: AVChannelLayout = core::mem::zeroed();
            av_channel_layout_default(&mut in_ch_layout, channels);

            let mut out_ch_layout: AVChannelLayout = core::mem::zeroed();
            av_channel_layout_copy(&mut out_ch_layout, &(*audio_codec_ctx).ch_layout);

            let err = swr_alloc_set_opts2(
                &mut swr_ctx,
                &out_ch_layout,
                sample_fmt,
                target_sample_rate,
                &in_ch_layout,
                AV_SAMPLE_FMT_FLTP,
                sample_rate,
                0,
                ptr::null_mut()
            );

            av_channel_layout_uninit(&mut in_ch_layout);
            av_channel_layout_uninit(&mut out_ch_layout);

            if err < 0 || swr_ctx.is_null() || swr_init(swr_ctx) < 0 {
                avcodec_free_context(&mut audio_codec_ctx as *mut _);
                avcodec_free_context(&mut codec_ctx as *mut _);
                avformat_free_context(fmt_ctx);
                return Err("Failed to initialize audio resampler".into());
            }

            let frame_size = if (*audio_codec_ctx).frame_size > 0 {
                (*audio_codec_ctx).frame_size
            } else {
                1024
            };

            audio_frame = av_frame_alloc();
            (*audio_frame).format = sample_fmt as i32;
            (*audio_frame).nb_samples = frame_size;
            av_channel_layout_copy(&mut (*audio_frame).ch_layout, &(*audio_codec_ctx).ch_layout);

            if av_frame_get_buffer(audio_frame, 0) < 0 {
                av_frame_free(&mut audio_frame as *mut _);
                avcodec_free_context(&mut audio_codec_ctx as *mut _);
                avcodec_free_context(&mut codec_ctx as *mut _);
                avformat_free_context(fmt_ctx);
                return Err("Failed to allocate audio frame buffer".into());
            }

            audio_buffer = vec![vec![0.0f32; 0]; channels as usize];
        }

        // --- File I/O Setup ---
        if ((*(*fmt_ctx).oformat).flags as u32 & AVFMT_NOFILE) == 0 {
            if avio_open(&mut (*fmt_ctx).pb, c_output_path.as_ptr(), AVIO_FLAG_WRITE as i32) < 0 {
                if audio_enabled {
                    av_frame_free(&mut audio_frame as *mut _);
                    avcodec_free_context(&mut audio_codec_ctx as *mut _);
                }
                avcodec_free_context(&mut codec_ctx as *mut _);
                avformat_free_context(fmt_ctx);
                return Err("Failed to open output file".into());
            }
        }

        let mut muxer_opts: *mut AVDictionary = ptr::null_mut();
        
        // Match Filtered Container Choices supporting AV1/VP9:
        // 0 -> Matroska Video (.mkv)
        // 1 -> MPEG-4 (.mp4) (Regular, faststart)
        // 2 -> Hybrid MP4 (.mp4) (Fragmented empty_moov)
        // 3 -> Fragmented MP4 (.mp4)
        // 4 -> QuickTime (.mov) (Regular, faststart)
        // 5 -> Hybrid MOV (.mov) (Fragmented empty_moov)
        // 6 -> Fragmented MOV (.mov)
        if container_choice == 2 || container_choice == 3 {
            // Hybrid/Fragmented MP4
            let opt_key = CString::new("movflags").unwrap();
            let opt_val = CString::new("empty_moov+default_base_moof+frag_keyframe").unwrap();
            av_dict_set(&mut muxer_opts, opt_key.as_ptr(), opt_val.as_ptr(), 0);
        } else if container_choice == 5 || container_choice == 6 {
            // Hybrid/Fragmented MOV
            let opt_key = CString::new("movflags").unwrap();
            let opt_val = CString::new("empty_moov+default_base_moof+frag_keyframe").unwrap();
            av_dict_set(&mut muxer_opts, opt_key.as_ptr(), opt_val.as_ptr(), 0);
        } else if container_choice == 1 || container_choice == 4 {
            // Regular MP4/MOV with faststart
            let opt_key = CString::new("movflags").unwrap();
            let opt_val = CString::new("faststart").unwrap();
            av_dict_set(&mut muxer_opts, opt_key.as_ptr(), opt_val.as_ptr(), 0);
        }

        let write_header_res = avformat_write_header(fmt_ctx, &mut muxer_opts);
        av_dict_free(&mut muxer_opts);

        if write_header_res < 0 {
            if audio_enabled {
                av_frame_free(&mut audio_frame as *mut _);
                avcodec_free_context(&mut audio_codec_ctx as *mut _);
            }
            avcodec_free_context(&mut codec_ctx as *mut _);
            avformat_free_context(fmt_ctx);
            return Err("Failed to write header".into());
        }

        let src_fmt = if colorspace == 1 || colorspace == 2 {
            AV_PIX_FMT_BGRA64LE
        } else {
            AV_PIX_FMT_BGRA
        };

        let sws_ctx = sws_getContext(
            width, height, src_fmt,
            width, height, dst_fmt,
            SWS_BILINEAR as i32, ptr::null_mut(), ptr::null_mut(), ptr::null()
        );

        let frame = av_frame_alloc();
        (*frame).format = dst_fmt as i32;
        (*frame).width = width;
        (*frame).height = height;
        av_frame_get_buffer(frame, 32);

        let pkt = av_packet_alloc();

        let temp_bgra64 = if colorspace == 1 || colorspace == 2 {
            vec![0u16; (width * height * 4) as usize]
        } else {
            Vec::new()
        };

        Ok(Self {
            fmt_ctx,
            codec_ctx,
            stream,
            sws_ctx,
            frame,
            pkt,
            video_pts: 0,
            colorspace,
            temp_bgra64,

            audio_codec_ctx,
            audio_stream,
            swr_ctx,
            audio_frame,
            audio_pts: 0,
            audio_channels: channels,
            audio_in_sample_rate: sample_rate,
            audio_buffer,
            has_audio: audio_enabled,
        })
    }

    pub unsafe fn encode_frame(
        &mut self,
        video_data: Option<(&[u8], i32)>,
        audio_data: Option<&[Vec<f32>]>,
    ) -> Result<(), String> {
        if let Some((bgra_data, row_bytes)) = video_data {
            let mut src_data: [*const u8; 4] = [ptr::null(); 4];
            let mut src_linesize: [i32; 4] = [0; 4];
            
            let height = (*self.codec_ctx).height;
            if self.colorspace == 1 || self.colorspace == 2 {
                let f32_len = bgra_data.len() / 4;
                let f32_slice = core::slice::from_raw_parts(bgra_data.as_ptr() as *const f32, f32_len);
                crate::plugin::shared::f32_to_bgra64(f32_slice, &mut self.temp_bgra64);
                
                // Flip vertically for HDR: point to start of last row and use negative stride
                let stride = (*self.codec_ctx).width * 8;
                src_data[0] = self.temp_bgra64.as_ptr().add(((height - 1) * (*self.codec_ctx).width * 4) as usize) as *const u8;
                src_linesize[0] = -stride;
            } else {
                // Flip vertically for SDR: point to start of last row and use negative stride
                src_data[0] = bgra_data.as_ptr().add(((height - 1) * row_bytes) as usize);
                src_linesize[0] = -row_bytes;
            }

            sws_scale(
                self.sws_ctx,
                src_data.as_ptr(),
                src_linesize.as_ptr(),
                0,
                height,
                (*self.frame).data.as_mut_ptr() as *mut *mut u8,
                (*self.frame).linesize.as_mut_ptr(),
            );

            (*self.frame).pts = self.video_pts;
            self.video_pts += 1;

            if avcodec_send_frame(self.codec_ctx, self.frame) < 0 {
                return Err("Error sending video frame to encoder".into());
            }

            while avcodec_receive_packet(self.codec_ctx, self.pkt) == 0 {
                av_packet_rescale_ts(self.pkt, (*self.codec_ctx).time_base, (*self.stream).time_base);
                (*self.pkt).stream_index = (*self.stream).index;
                av_interleaved_write_frame(self.fmt_ctx, self.pkt);
                av_packet_unref(self.pkt);
            }
        }

        if let Some(audio_samples) = audio_data {
            if self.has_audio && !self.audio_codec_ctx.is_null() && !self.swr_ctx.is_null() {
                self.resample_and_buffer_audio(audio_samples)?;

                let frame_size = (*self.audio_codec_ctx).frame_size as usize;
                while self.audio_buffer[0].len() >= frame_size {
                    for ch in 0..self.audio_channels as usize {
                        let dest = (*self.audio_frame).data[ch] as *mut f32;
                        let src = &self.audio_buffer[ch][..frame_size];
                        ptr::copy_nonoverlapping(src.as_ptr(), dest, frame_size);
                    }

                    (*self.audio_frame).pts = self.audio_pts;
                    self.audio_pts += frame_size as i64;

                    if avcodec_send_frame(self.audio_codec_ctx, self.audio_frame) < 0 {
                        return Err("Error sending audio frame to encoder".into());
                    }

                    let pkt = av_packet_alloc();
                    while avcodec_receive_packet(self.audio_codec_ctx, pkt) == 0 {
                        av_packet_rescale_ts(pkt, (*self.audio_codec_ctx).time_base, (*self.audio_stream).time_base);
                        (*pkt).stream_index = (*self.audio_stream).index;
                        av_interleaved_write_frame(self.fmt_ctx, pkt);
                        av_packet_unref(pkt);
                    }
                    av_packet_free(&mut (pkt as *mut _));

                    for ch in 0..self.audio_channels as usize {
                        self.audio_buffer[ch].drain(0..frame_size);
                    }
                }
            }
        }

        Ok(())
    }

    unsafe fn resample_and_buffer_audio(&mut self, audio_samples: &[Vec<f32>]) -> Result<(), String> {
        let nb_samples = audio_samples[0].len();
        if nb_samples == 0 {
            return Ok(());
        }

        let mut in_ptrs: Vec<*const u8> = audio_samples.iter().map(|ch| ch.as_ptr() as *const u8).collect();
        let max_out_samples = swr_get_out_samples(self.swr_ctx, nb_samples as i32);
        if max_out_samples <= 0 {
            return Ok(());
        }

        let mut out_buffers: Vec<Vec<f32>> = vec![vec![0.0; max_out_samples as usize]; self.audio_channels as usize];
        let mut out_ptrs: Vec<*mut u8> = out_buffers.iter_mut().map(|ch| ch.as_mut_ptr() as *mut u8).collect();

        let converted = swr_convert(
            self.swr_ctx,
            out_ptrs.as_mut_ptr(),
            max_out_samples,
            in_ptrs.as_mut_ptr(),
            nb_samples as i32,
        );

        if converted < 0 {
            return Err(format!("Audio resampling failed: {}", converted));
        }

        for ch in 0..self.audio_channels as usize {
            self.audio_buffer[ch].extend_from_slice(&out_buffers[ch][..converted as usize]);
        }

        Ok(())
    }

    pub unsafe fn finish(&mut self) -> Result<(), String> {
        if !self.codec_ctx.is_null() {
            avcodec_send_frame(self.codec_ctx, ptr::null());
            while avcodec_receive_packet(self.codec_ctx, self.pkt) == 0 {
                av_packet_rescale_ts(self.pkt, (*self.codec_ctx).time_base, (*self.stream).time_base);
                (*self.pkt).stream_index = (*self.stream).index;
                av_interleaved_write_frame(self.fmt_ctx, self.pkt);
                av_packet_unref(self.pkt);
            }
        }

        if self.has_audio && !self.audio_codec_ctx.is_null() {
            let frame_size = (*self.audio_codec_ctx).frame_size as usize;
            let remaining = self.audio_buffer[0].len();
            if remaining > 0 && remaining < frame_size {
                for ch in 0..self.audio_channels as usize {
                    self.audio_buffer[ch].resize(frame_size, 0.0);
                }
                for ch in 0..self.audio_channels as usize {
                    let dest = (*self.audio_frame).data[ch] as *mut f32;
                    let src = &self.audio_buffer[ch][..frame_size];
                    ptr::copy_nonoverlapping(src.as_ptr(), dest, frame_size);
                }
                (*self.audio_frame).pts = self.audio_pts;
                self.audio_pts += frame_size as i64;

                if avcodec_send_frame(self.audio_codec_ctx, self.audio_frame) == 0 {
                    let pkt = av_packet_alloc();
                    while avcodec_receive_packet(self.audio_codec_ctx, pkt) == 0 {
                        av_packet_rescale_ts(pkt, (*self.audio_codec_ctx).time_base, (*self.audio_stream).time_base);
                        (*pkt).stream_index = (*self.audio_stream).index;
                        av_interleaved_write_frame(self.fmt_ctx, pkt);
                        av_packet_unref(pkt);
                    }
                    av_packet_free(&mut (pkt as *mut _));
                }
            }

            avcodec_send_frame(self.audio_codec_ctx, ptr::null());
            let pkt = av_packet_alloc();
            while avcodec_receive_packet(self.audio_codec_ctx, pkt) == 0 {
                av_packet_rescale_ts(pkt, (*self.audio_codec_ctx).time_base, (*self.audio_stream).time_base);
                (*pkt).stream_index = (*self.audio_stream).index;
                av_interleaved_write_frame(self.fmt_ctx, pkt);
                av_packet_unref(pkt);
            }
            av_packet_free(&mut (pkt as *mut _));
        }

        av_write_trailer(self.fmt_ctx);
        Ok(())
    }
}

impl Drop for FFmpegEncoder {
    fn drop(&mut self) {
        unsafe {
            if !self.codec_ctx.is_null() {
                avcodec_free_context(&mut self.codec_ctx);
            }
            if !self.audio_codec_ctx.is_null() {
                avcodec_free_context(&mut self.audio_codec_ctx);
            }
            if !self.fmt_ctx.is_null() {
                if ((*(*self.fmt_ctx).oformat).flags as u32 & AVFMT_NOFILE) == 0 {
                    avio_closep(&mut (*self.fmt_ctx).pb);
                }
                avformat_free_context(self.fmt_ctx);
            }
            if !self.sws_ctx.is_null() {
                sws_freeContext(self.sws_ctx);
            }
            if !self.swr_ctx.is_null() {
                swr_free(&mut self.swr_ctx);
            }
            if !self.frame.is_null() {
                av_frame_free(&mut self.frame);
            }
            if !self.audio_frame.is_null() {
                av_frame_free(&mut self.audio_frame);
            }
            if !self.pkt.is_null() {
                av_packet_free(&mut self.pkt);
            }
        }
    }
}
