use std::ffi::{c_void, CString};
use std::ptr;
use crate::*;
use crate::ffmpeg_ffi::*;
use crate::importer::utils::get_utf16_string;

use std::sync::{Mutex, OnceLock};
use std::collections::HashMap;

#[derive(Clone)]
pub struct CachedMetadata {
    pub width: i32,
    pub height: i32,
    pub bit_depth: u8,
    pub has_audio: bool,
    pub audio_channels: i32,
    pub audio_sample_rate: f32,
    pub frame_rate: i64,
    pub vid_duration: i32,
    pub aud_duration: i64,
}

static METADATA_CACHE: OnceLock<Mutex<HashMap<String, CachedMetadata>>> = OnceLock::new();

pub unsafe fn handle_get_info8(_std_parms: *mut crate::imStdParms, param1: *mut c_void, param2: *mut c_void) -> prMALError {
    let file_access = param1 as *mut imFileAccessRec8;
    let file_info = param2 as *mut imFileInfoRec8;
    if file_access.is_null() || file_info.is_null() {
        return malUnknownError as prMALError;
    }
    let file_access = &mut *file_access;
    let file_info = &mut *file_info;

    let path_str = get_utf16_string(file_access.filepath);

    let cache = METADATA_CACHE.get_or_init(|| Mutex::new(HashMap::new()));
    if let Ok(guard) = cache.lock() {
        if let Some(cached) = guard.get(&path_str) {
            crate::log_debug!("handle_get_info8: METADATA CACHE HIT for {}", path_str);
            file_info.hasVideo = 1;
            file_info.vidInfo.imageWidth = cached.width;
            file_info.vidInfo.imageHeight = cached.height;
            file_info.vidInfo.depth = if cached.bit_depth > 8 { 40 } else { 32 };
            file_info.vidInfo.frameRate = cached.frame_rate;
            file_info.vidDuration = cached.vid_duration;
            file_info.hasAudio = if cached.has_audio { 1 } else { 0 };
            if cached.has_audio {
                file_info.audInfo.numChannels = cached.audio_channels;
                file_info.audInfo.sampleRate = cached.audio_sample_rate;
                file_info.audInfo.sampleType = PrAudioSampleType_kPrAudioSampleType_32BitFloat;
                file_info.audDuration = cached.aud_duration;
            } else {
                file_info.audDuration = 0;
            }
            file_info.accessModes = kRandomAccessImport as i32;
            file_info.vidInfo.supportsAsyncIO = 1;
            file_info.vidInfo.supportsGetSourceVideo = 1;
            return malNoError as prMALError;
        }
    }

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
    let mut audio_stream_idx = -1;
    for i in 0..(*fmt_ctx).nb_streams {
        let stream = *(*fmt_ctx).streams.add(i as usize);
        let codec_par = (*stream).codecpar;
        if (*codec_par).codec_type == AVMEDIA_TYPE_VIDEO && video_stream_idx == -1 {
            video_stream_idx = i as i32;
        } else if (*codec_par).codec_type == AVMEDIA_TYPE_AUDIO && audio_stream_idx == -1 {
            audio_stream_idx = i as i32;
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

    crate::log_debug!("Importer info8 queried file: {}, codec: {} ({})", path_str, codec_name, codec_id);

    if codec_id != AV_CODEC_ID_AV1 && codec_id != AV_CODEC_ID_VP9 && codec_id != AV_CODEC_ID_VP8 {
        crate::log_debug!("Importer info8 rejected file: codec not AV1/VP9/VP8");
        avformat_close_input(&mut fmt_ctx);
        return PrImporterReturnValue_imBadFile as prMALError;
    }

    let pix_fmt = (*video_codec_par).format;
    let desc = av_pix_fmt_desc_get(pix_fmt);
    let mut bit_depth = 8;
    if !desc.is_null() {
        bit_depth = (*desc).comp[0].depth;
    }

    file_info.hasVideo = 1;
    file_info.vidInfo.imageWidth = (*video_codec_par).width as i32;
    file_info.vidInfo.imageHeight = (*video_codec_par).height as i32;
    if bit_depth > 8 {
        file_info.vidInfo.depth = 40;
    } else {
        file_info.vidInfo.depth = 32;
    }

    let fps_num = (*video_stream).avg_frame_rate.num as i64;
    let fps_den = (*video_stream).avg_frame_rate.den as i64;
    if fps_den > 0 && fps_num > 0 {
        file_info.vidInfo.frameRate = (254016000000_i64 * fps_den) / fps_num;
    } else {
        file_info.vidInfo.frameRate = 254016000000_i64 / 24;
    }

    let fps = if fps_den > 0 && fps_num > 0 { fps_num as f64 / fps_den as f64 } else { 24.0 };

    let video_time_base = (*video_stream).time_base;
    let video_duration_ts = (*video_stream).duration;
    let duration_sec = if video_duration_ts > 0 {
        video_duration_ts as f64 * (video_time_base.num as f64 / video_time_base.den as f64)
    } else if (*fmt_ctx).duration > 0 {
        (*fmt_ctx).duration as f64 / AV_TIME_BASE as f64
    } else {
        0.0
    };

    if duration_sec > 0.0 {
        let frames = duration_sec * fps;
        file_info.vidDuration = frames as i32;
    } else {
        file_info.vidDuration = 0;
    }

    if audio_stream_idx != -1 {
        let audio_stream = *(*fmt_ctx).streams.add(audio_stream_idx as usize);
        let audio_codec_par = (*audio_stream).codecpar;
        
        file_info.hasAudio = 1;
        file_info.audInfo.numChannels = (*audio_codec_par).ch_layout.nb_channels;
        file_info.audInfo.sampleRate = (*audio_codec_par).sample_rate as f32;
        file_info.audInfo.sampleType = PrAudioSampleType_kPrAudioSampleType_32BitFloat;
        
        let audio_time_base = (*audio_stream).time_base;
        let audio_duration_ts = (*audio_stream).duration;
        let aud_dur_sec = if audio_duration_ts > 0 {
            audio_duration_ts as f64 * (audio_time_base.num as f64 / audio_time_base.den as f64)
        } else if duration_sec > 0.0 {
            duration_sec
        } else {
            0.0
        };
        file_info.audDuration = (aud_dur_sec * (*audio_codec_par).sample_rate as f64) as i64;
    } else {
        file_info.hasAudio = 0;
        file_info.audDuration = 0;
    }

    file_info.accessModes = kRandomAccessImport as i32;
    file_info.vidInfo.supportsAsyncIO = 1;
    file_info.vidInfo.supportsGetSourceVideo = 1;

    let metadata = CachedMetadata {
        width: file_info.vidInfo.imageWidth,
        height: file_info.vidInfo.imageHeight,
        bit_depth: bit_depth as u8,
        has_audio: file_info.hasAudio != 0,
        audio_channels: file_info.audInfo.numChannels,
        audio_sample_rate: file_info.audInfo.sampleRate,
        frame_rate: file_info.vidInfo.frameRate,
        vid_duration: file_info.vidDuration,
        aud_duration: file_info.audDuration,
    };
    if let Ok(mut guard) = cache.lock() {
        guard.insert(path_str, metadata);
    }

    avformat_close_input(&mut fmt_ctx);
    malNoError as prMALError
}

pub unsafe fn handle_get_info9(std_parms: *mut crate::imStdParms, param1: *mut c_void, param2: *mut c_void) -> prMALError {
    let file_info9 = param2 as *mut imFileInfoRec9;
    if file_info9.is_null() {
        return malUnknownError as prMALError;
    }
    let file_info8 = std::ptr::addr_of_mut!((*file_info9).info) as *mut imFileInfoRec8;
    handle_get_info8(std_parms, param1, file_info8 as *mut c_void)
}
