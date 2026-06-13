use std::ffi::{c_void, CString};
use std::ptr;
use crate::*;
use crate::ffmpeg_ffi::*;
use crate::importer::types::*;
use crate::importer::utils::get_utf16_string;
use std::sync::{Arc, Mutex, Condvar};
use std::collections::HashSet;

pub unsafe extern "C" fn get_format(
    ctx: *mut AVCodecContext,
    pix_fmts: *const AVPixelFormat,
) -> AVPixelFormat {
    let mut offered = Vec::new();
    let mut i = 0;
    while *pix_fmts.add(i) != AV_PIX_FMT_NONE {
        let fmt = *pix_fmts.add(i);
        offered.push(fmt);
        i += 1;
    }
    crate::log_debug!("get_format: offered formats = {:?}", offered);
    
    let mut i = 0;
    while *pix_fmts.add(i) != AV_PIX_FMT_NONE {
        let fmt = *pix_fmts.add(i);
        if fmt == AV_PIX_FMT_D3D11 
            || fmt == AV_PIX_FMT_D3D11VA_VLD 
            || fmt == AV_PIX_FMT_DXVA2_VLD 
            || fmt == AV_PIX_FMT_CUDA 
        {
            if !(*ctx).hw_device_ctx.is_null() {
                crate::log_debug!("get_format: SELECTED HW FORMAT = {}", fmt);
                return fmt;
            }
        }
        i += 1;
    }
    *pix_fmts
}

pub unsafe fn handle_open_file8(std_parms: *mut imStdParms, param1: *mut c_void, param2: *mut c_void) -> prMALError {
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
    
    crate::log_debug!("Importer opened file: {}, video codec: {} ({})", path_str, codec_name, codec_id);

    if codec_id != AV_CODEC_ID_AV1 && codec_id != AV_CODEC_ID_VP9 && codec_id != AV_CODEC_ID_VP8 {
        crate::log_debug!("Importer rejected file: codec not AV1/VP9/VP8");
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

    let video_codec_par = (*(*(*fmt_ctx).streams.add(video_stream_idx as usize))).codecpar;
    if avcodec_parameters_to_context(codec_ctx, video_codec_par) < 0 {
        avcodec_free_context(&mut codec_ctx);
        avformat_close_input(&mut fmt_ctx);
        return malUnknownError as prMALError;
    }

    // Configure multi-threaded slice decoding (zero latency, high CPU parallelization)
    (*codec_ctx).thread_count = 0; // Auto-detect core count
    (*codec_ctx).thread_type = crate::ffmpeg_ffi::FF_THREAD_SLICE as i32;

    let mut hw_device_ctx: *mut AVBufferRef = ptr::null_mut();
    let mut chosen_hw_name = "disabled (software fallback)";

    // Try initializing D3D11VA
    let mut err = av_hwdevice_ctx_create(
        &mut hw_device_ctx,
        AV_HWDEVICE_TYPE_D3D11VA,
        ptr::null(),
        ptr::null_mut(),
        0
    );

    if err == 0 {
        chosen_hw_name = "D3D11VA";
    } else {
        // Try CUDA
        err = av_hwdevice_ctx_create(
            &mut hw_device_ctx,
            AV_HWDEVICE_TYPE_CUDA,
            ptr::null(),
            ptr::null_mut(),
            0
        );
        if err == 0 {
            chosen_hw_name = "CUDA";
        }
    }

    if !hw_device_ctx.is_null() {
        (*codec_ctx).hw_device_ctx = av_buffer_ref(hw_device_ctx);
        (*codec_ctx).get_format = Some(get_format);
    }

    crate::log_debug!("Selected HW Decoder: {}", chosen_hw_name);

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

    let ffmpeg_ctx = Arc::new(Mutex::new(FFmpegContext {
        format_ctx: fmt_ctx,
        codec_ctx,
        frame,
        packet,
        audio_codec_ctx,
        audio_frame,
        swr_ctx,
    }));

    let cache = Arc::new((
        Mutex::new(CacheState {
            frame_cache: Vec::new(),
            decoding_in_progress: HashSet::new(),
        }),
        Condvar::new(),
    ));

    let (worker_tx, rx) = std::sync::mpsc::channel();
    
    // Spawn background worker thread
    let ffmpeg_clone = Arc::clone(&ffmpeg_ctx);
    let cache_clone = Arc::clone(&cache);
    let video_stream_idx_val = video_stream_idx;
    
    let worker_thread = std::thread::spawn(move || {
        crate::importer::image::worker_thread_loop(ffmpeg_clone, video_stream_idx_val, cache_clone, rx);
    });

    let importer_data = Box::new(ImporterData {
        ffmpeg: ffmpeg_ctx,
        video_stream_idx,
        audio_stream_idx,
        cache,
        worker_tx,
        worker_thread: Some(worker_thread),
        audio_buffer,
        audio_buffer_start_sample: 0,
        needs_first_pts: false,
        hw_device_ctx,
        last_decoded_frame: Mutex::new(-999999),
        temp_bgra64_buffer: Mutex::new(Vec::new()),
        std_parms,
        async_data_ptr: std::ptr::null_mut(),
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

