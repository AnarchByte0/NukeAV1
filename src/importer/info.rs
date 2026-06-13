use std::ffi::{c_void, CString};
use std::ptr;
use crate::*;
use crate::ffmpeg_ffi::*;
use crate::importer::utils::get_utf16_string;

pub unsafe fn handle_get_info8(param1: *mut c_void, param2: *mut c_void) -> prMALError {
    let file_access = param1 as *mut imFileAccessRec8;
    let file_info = param2 as *mut imFileInfoRec8;
    if file_access.is_null() || file_info.is_null() {
        return malUnknownError as prMALError;
    }
    let file_access = &mut *file_access;
    let file_info = &mut *file_info;

    let path_str = get_utf16_string(file_access.filepath);
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

    if let Ok(mut file) = std::fs::OpenOptions::new().create(true).append(true).open("C:\\Users\\maksi\\NukeAV1_debug.log") {
        use std::io::Write;
        let _ = writeln!(file, "Importer info8 queried file: {}, codec: {} ({})", path_str, codec_name, codec_id);
    }

    if codec_id != AV_CODEC_ID_AV1 && codec_id != AV_CODEC_ID_VP9 && codec_id != AV_CODEC_ID_VP8 {
        if let Ok(mut file) = std::fs::OpenOptions::new().create(true).append(true).open("C:\\Users\\maksi\\NukeAV1_debug.log") {
            use std::io::Write;
            let _ = writeln!(file, "Importer info8 rejected file: codec not AV1/VP9/VP8");
        }
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
    if video_duration_ts > 0 {
        let duration_sec = video_duration_ts as f64 * (video_time_base.num as f64 / video_time_base.den as f64);
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
        if audio_duration_ts > 0 {
            let duration_sec = audio_duration_ts as f64 * (audio_time_base.num as f64 / audio_time_base.den as f64);
            file_info.audDuration = (duration_sec * (*audio_codec_par).sample_rate as f64) as i64;
        } else if file_info.vidDuration > 0 {
            let duration_sec = file_info.vidDuration as f64 / fps;
            file_info.audDuration = (duration_sec * (*audio_codec_par).sample_rate as f64) as i64;
        } else {
            file_info.audDuration = 0;
        }
    } else {
        file_info.hasAudio = 0;
        file_info.audDuration = 0;
    }

    file_info.accessModes = kRandomAccessImport as i32;

    avformat_close_input(&mut fmt_ctx);
    malNoError as prMALError
}

pub unsafe fn handle_get_info9(param1: *mut c_void, param2: *mut c_void) -> prMALError {
    let file_info9 = param2 as *mut imFileInfoRec9;
    if file_info9.is_null() {
        return malUnknownError as prMALError;
    }
    let file_info8 = std::ptr::addr_of_mut!((*file_info9).info) as *mut imFileInfoRec8;
    handle_get_info8(param1, file_info8 as *mut c_void)
}
