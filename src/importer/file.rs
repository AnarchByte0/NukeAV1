use std::ffi::{c_void, CString};
use std::ptr;
use crate::*;
use crate::ffmpeg_ffi::*;
use crate::importer::types::ImporterData;
use crate::importer::utils::get_utf16_string;

pub unsafe extern "C" fn get_format(
    ctx: *mut AVCodecContext,
    pix_fmts: *const AVPixelFormat,
) -> AVPixelFormat {
    let mut i = 0;
    while *pix_fmts.add(i) != AV_PIX_FMT_NONE {
        let fmt = *pix_fmts.add(i);
        if fmt == AV_PIX_FMT_D3D11 
            || fmt == AV_PIX_FMT_D3D11VA_VLD 
            || fmt == AV_PIX_FMT_DXVA2_VLD 
            || fmt == AV_PIX_FMT_CUDA 
        {
            if !(*ctx).hw_device_ctx.is_null() {
                return fmt;
            }
        }
        i += 1;
    }
    *pix_fmts
}

pub unsafe fn handle_open_file8(param1: *mut c_void, param2: *mut c_void) -> prMALError {
    let file_ref_ptr = param1 as *mut *mut c_void;
    let file_open = param2 as *mut imFileOpenRec8;
    if file_open.is_null() {
        return malUnknownError as prMALError;
    }
    let file_open = &mut *file_open;
    let file_info = &(*file_open).fileinfo;

    let path_str = get_utf16_string(file_info.filepath);
    let path_c = CString::new(path_str.clone()).unwrap();

    let mut fmt_ctx: *mut AVFormatContext = ptr::null_mut();
    if avformat_open_input(&mut fmt_ctx, path_c.as_ptr(), ptr::null_mut(), ptr::null_mut()) != 0 {
        return malUnknownError as prMALError;
    }

    if avformat_find_stream_info(fmt_ctx, ptr::null_mut()) < 0 {
        avformat_close_input(&mut fmt_ctx);
        return malUnknownError as prMALError;
    }

    let mut video_stream_idx = -1;
    for i in 0..(*fmt_ctx).nb_streams {
        let stream = *(*fmt_ctx).streams.add(i as usize);
        if (*(*stream).codecpar).codec_type == AVMEDIA_TYPE_VIDEO {
            video_stream_idx = i as i32;
            break;
        }
    }

    if video_stream_idx == -1 {
        avformat_close_input(&mut fmt_ctx);
        return malUnknownError as prMALError;
    }

    let video_stream = *(*fmt_ctx).streams.add(video_stream_idx as usize);
    let video_codec_par = (*video_stream).codecpar;
    
    let codec_id = (*video_codec_par).codec_id;
    let codec_name_ptr = avcodec_get_name(codec_id);
    let codec_name = if !codec_name_ptr.is_null() {
        std::ffi::CStr::from_ptr(codec_name_ptr).to_string_lossy().into_owned()
    } else {
        "unknown".to_string()
    };
    
    // Write debug logs even in release for this troubleshooting phase
    if let Ok(mut file) = std::fs::OpenOptions::new().create(true).append(true).open("C:\\Users\\maksi\\NukeAV1_debug.log") {
        use std::io::Write;
        let _ = writeln!(file, "Importer opened file: {}, video codec: {} ({})", path_str, codec_name, codec_id);
    }

    if codec_id != AV_CODEC_ID_AV1 && codec_id != AV_CODEC_ID_VP9 && codec_id != AV_CODEC_ID_VP8 {
        if let Ok(mut file) = std::fs::OpenOptions::new().create(true).append(true).open("C:\\Users\\maksi\\NukeAV1_debug.log") {
            use std::io::Write;
            let _ = writeln!(file, "Importer rejected file: codec not AV1/VP9/VP8");
        }
        avformat_close_input(&mut fmt_ctx);
        return PrImporterReturnValue_imBadFile as prMALError;
    }

    let codec = avcodec_find_decoder(codec_id);
    if codec.is_null() {
        avformat_close_input(&mut fmt_ctx);
        return malUnknownError as prMALError;
    }

    let mut codec_ctx = avcodec_alloc_context3(codec);
    if codec_ctx.is_null() {
        avformat_close_input(&mut fmt_ctx);
        return malUnknownError as prMALError;
    }

    if avcodec_parameters_to_context(codec_ctx, video_codec_par) < 0 {
        avcodec_free_context(&mut codec_ctx);
        avformat_close_input(&mut fmt_ctx);
        return malUnknownError as prMALError;
    }

    let mut hw_device_ctx: *mut AVBufferRef = ptr::null_mut();
    let hw_types = [
        AV_HWDEVICE_TYPE_D3D11VA,
        AV_HWDEVICE_TYPE_DXVA2,
        AV_HWDEVICE_TYPE_CUDA,
    ];
    for &hw_type in &hw_types {
        let mut ctx_ptr: *mut AVBufferRef = ptr::null_mut();
        let err = av_hwdevice_ctx_create(
            &mut ctx_ptr,
            hw_type,
            ptr::null(),
            ptr::null_mut(),
            0
        );
        if err >= 0 && !ctx_ptr.is_null() {
            hw_device_ctx = ctx_ptr;
            (*codec_ctx).hw_device_ctx = av_buffer_ref(hw_device_ctx);
            (*codec_ctx).get_format = Some(get_format);
            break;
        }
    }

    if avcodec_open2(codec_ctx, codec, ptr::null_mut()) < 0 {
        if !hw_device_ctx.is_null() {
            av_buffer_unref(&mut hw_device_ctx);
        }
        avcodec_free_context(&mut codec_ctx);
        avformat_close_input(&mut fmt_ctx);
        return malUnknownError as prMALError;
    }

    // Audio stream discovery
    let mut audio_stream_idx = -1;
    for i in 0..(*fmt_ctx).nb_streams {
        let stream = *(*fmt_ctx).streams.add(i as usize);
        if (*(*stream).codecpar).codec_type == AVMEDIA_TYPE_AUDIO {
            audio_stream_idx = i as i32;
            break;
        }
    }

    let mut audio_codec_ctx: *mut AVCodecContext = ptr::null_mut();
    let mut audio_frame: *mut AVFrame = ptr::null_mut();
    let mut swr_ctx: *mut SwrContext = ptr::null_mut();
    let mut audio_buffer = Vec::new();

    if audio_stream_idx != -1 {
        let audio_stream = *(*fmt_ctx).streams.add(audio_stream_idx as usize);
        let audio_codec_par = (*audio_stream).codecpar;
        let audio_codec = avcodec_find_decoder((*audio_codec_par).codec_id);
        if !audio_codec.is_null() {
            audio_codec_ctx = avcodec_alloc_context3(audio_codec);
            if !audio_codec_ctx.is_null() {
                if avcodec_parameters_to_context(audio_codec_ctx, audio_codec_par) >= 0 {
                    if avcodec_open2(audio_codec_ctx, audio_codec, ptr::null_mut()) >= 0 {
                        audio_frame = av_frame_alloc();
                        
                        let in_sample_fmt = (*audio_codec_ctx).sample_fmt;
                        let out_sample_fmt = AV_SAMPLE_FMT_FLTP;
                        let sample_rate = (*audio_codec_ctx).sample_rate;
                        let channels = (*audio_codec_ctx).ch_layout.nb_channels;
                        
                        let in_ch_layout = (*audio_codec_ctx).ch_layout;
                        let out_ch_layout = (*audio_codec_ctx).ch_layout;
                        
                        let err = swr_alloc_set_opts2(
                            &mut swr_ctx,
                            &out_ch_layout,
                            out_sample_fmt,
                            sample_rate,
                            &in_ch_layout,
                            in_sample_fmt,
                            sample_rate,
                            0,
                            ptr::null_mut()
                        );
                        if err < 0 || swr_ctx.is_null() || swr_init(swr_ctx) < 0 {
                            if !swr_ctx.is_null() {
                                swr_free(&mut swr_ctx);
                                swr_ctx = ptr::null_mut();
                            }
                        }
                        
                        audio_buffer = vec![Vec::new(); channels as usize];
                    } else {
                        avcodec_free_context(&mut audio_codec_ctx);
                        audio_codec_ctx = ptr::null_mut();
                    }
                } else {
                    avcodec_free_context(&mut audio_codec_ctx);
                    audio_codec_ctx = ptr::null_mut();
                }
            }
        }
    }

    let frame = av_frame_alloc();
    let packet = av_packet_alloc();

    let importer_data = Box::new(ImporterData {
        format_ctx: fmt_ctx,
        codec_ctx,
        video_stream_idx,
        frame,
        packet,
        audio_stream_idx,
        audio_codec_ctx,
        audio_frame,
        swr_ctx,
        audio_buffer,
        audio_buffer_start_sample: 0,
        needs_first_pts: false,
        hw_device_ctx,
    });

    let raw_data = Box::into_raw(importer_data);
    file_open.privatedata = raw_data as *mut c_void;
    *file_ref_ptr = raw_data as *mut c_void;

    malNoError as prMALError
}

pub unsafe fn handle_quiet_file(_param1: *mut c_void) -> prMALError {
    // We do not free the private data here, as it will be freed in handle_close_file.
    malNoError as prMALError
}

pub unsafe fn handle_close_file(param1: *mut c_void) -> prMALError {
    let file_ref = param1;
    if !file_ref.is_null() {
        let _ = Box::from_raw(file_ref as *mut ImporterData);
    }
    malNoError as prMALError
}
