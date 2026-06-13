use std::ffi::c_void;
use std::ptr;
use crate::*;
use crate::ffmpeg_ffi::*;
use crate::importer::types::ImporterData;

pub const AV_NOPTS_VALUE: i64 = i64::MIN;

pub unsafe fn handle_import_audio(param1: *mut c_void, param2: *mut c_void) -> prMALError {
    let file_ref = param1;
    if file_ref.is_null() {
        return malUnknownError as prMALError;
    }
    let importer_data = &mut *(file_ref as *mut ImporterData);

    let import_audio = param2 as *mut imImportAudioRec7;
    if import_audio.is_null() {
        return malUnknownError as prMALError;
    }
    let import_audio = &mut *import_audio;

    let ffmpeg = importer_data.ffmpeg.lock().unwrap();
    if ffmpeg.audio_codec_ctx.is_null() {
        return malUnknownError as prMALError;
    }

    let position = import_audio.position;
    let size = import_audio.size;
    let buffer = import_audio.buffer;

    let num_channels = (*ffmpeg.audio_codec_ctx).ch_layout.nb_channels as usize;

    // Check if we need to seek
    let in_buffer_range = position >= importer_data.audio_buffer_start_sample
        && position < importer_data.audio_buffer_start_sample + importer_data.audio_buffer[0].len() as i64;

    if !in_buffer_range {
        let audio_stream = *(*ffmpeg.format_ctx).streams.add(importer_data.audio_stream_idx as usize);
        let time_base = (*audio_stream).time_base;
        let sample_rate = (*ffmpeg.audio_codec_ctx).sample_rate as i64;
        
        let target_pts = if time_base.num > 0 && sample_rate > 0 {
            (position * time_base.den as i64) / (sample_rate * time_base.num as i64)
        } else {
            0
        };

        av_seek_frame(
            ffmpeg.format_ctx,
            importer_data.audio_stream_idx,
            target_pts,
            AVSEEK_FLAG_BACKWARD as i32,
        );
        avcodec_flush_buffers(ffmpeg.audio_codec_ctx);

        for ch in 0..num_channels {
            importer_data.audio_buffer[ch].clear();
        }
        importer_data.audio_buffer_start_sample = position;
        importer_data.needs_first_pts = true;
    } else {
        let discard = (position - importer_data.audio_buffer_start_sample) as usize;
        if discard > 0 {
            for ch in 0..num_channels {
                importer_data.audio_buffer[ch].drain(0..discard);
            }
            importer_data.audio_buffer_start_sample = position;
        }
    }

    let sample_rate = (*ffmpeg.audio_codec_ctx).sample_rate as i64;
    let audio_stream = *(*ffmpeg.format_ctx).streams.add(importer_data.audio_stream_idx as usize);
    let time_base = (*audio_stream).time_base;

    while importer_data.audio_buffer[0].len() < size as usize {
        if av_read_frame(ffmpeg.format_ctx, ffmpeg.packet) < 0 {
            break;
        }

        if (*ffmpeg.packet).stream_index == importer_data.audio_stream_idx {
            if avcodec_send_packet(ffmpeg.audio_codec_ctx, ffmpeg.packet) == 0 {
                while avcodec_receive_frame(ffmpeg.audio_codec_ctx, ffmpeg.audio_frame) == 0 {
                    let frame = ffmpeg.audio_frame;

                    if importer_data.needs_first_pts {
                        let pts = if (*frame).pts != AV_NOPTS_VALUE {
                            (*frame).pts
                        } else {
                            (*frame).best_effort_timestamp
                        };
                        if pts != AV_NOPTS_VALUE && time_base.den > 0 && sample_rate > 0 {
                            let actual_start_sample = (pts * sample_rate * time_base.num as i64) / time_base.den as i64;
                            importer_data.audio_buffer_start_sample = actual_start_sample;
                        }
                        importer_data.needs_first_pts = false;
                    }

                    if importer_data.audio_buffer_start_sample > position {
                        let gap = (importer_data.audio_buffer_start_sample - position) as usize;
                        for ch in 0..num_channels {
                            let mut zeros = vec![0.0f32; gap];
                            zeros.extend(&importer_data.audio_buffer[ch]);
                            importer_data.audio_buffer[ch] = zeros;
                        }
                        importer_data.audio_buffer_start_sample = position;
                    }

                    let nb_samples = (*frame).nb_samples;
                    let out_samples = if !ffmpeg.swr_ctx.is_null() {
                        swr_get_out_samples(ffmpeg.swr_ctx, nb_samples)
                    } else {
                        nb_samples
                    };

                    let mut resampled_channels = vec![vec![0.0f32; out_samples as usize]; num_channels];

                    if !ffmpeg.swr_ctx.is_null() {
                        let mut out_ptrs: Vec<*mut u8> = resampled_channels.iter_mut().map(|ch| ch.as_mut_ptr() as *mut u8).collect();
                        let mut in_ptrs: Vec<*const u8> = (0..num_channels).map(|ch| (*frame).data[ch] as *const u8).collect();

                        let converted = swr_convert(
                            ffmpeg.swr_ctx,
                            out_ptrs.as_mut_ptr(),
                            out_samples,
                            in_ptrs.as_mut_ptr(),
                            nb_samples,
                        );

                        if converted > 0 {
                            for ch in 0..num_channels {
                                importer_data.audio_buffer[ch].extend_from_slice(&resampled_channels[ch][..converted as usize]);
                            }
                        }
                    } else {
                        for ch in 0..num_channels {
                            let data_ptr = (*frame).data[ch] as *const f32;
                            let slice = core::slice::from_raw_parts(data_ptr, nb_samples as usize);
                            importer_data.audio_buffer[ch].extend_from_slice(slice);
                        }
                    }
                }
            }
        }
        av_packet_unref(ffmpeg.packet);
    }

    let copy_size = std::cmp::min(size as usize, importer_data.audio_buffer[0].len());

    for ch in 0..num_channels {
        let dest = *buffer.add(ch);
        if copy_size > 0 {
            let src = &importer_data.audio_buffer[ch][..copy_size];
            ptr::copy_nonoverlapping(src.as_ptr(), dest, copy_size);
        }

        if copy_size < size as usize {
            let remaining = size as usize - copy_size;
            let dest_remaining = dest.add(copy_size);
            ptr::write_bytes(dest_remaining, 0, remaining);
        }
    }

    if copy_size > 0 {
        for ch in 0..num_channels {
            importer_data.audio_buffer[ch].drain(0..copy_size);
        }
    }
    importer_data.audio_buffer_start_sample += copy_size as i64;

    malNoError as prMALError
}

pub unsafe fn handle_get_peak_audio(file_ref: *mut c_void, param: *mut c_void) -> prMALError {
    let peak_rec = param as *mut imPeakAudioRec;
    if peak_rec.is_null() || file_ref.is_null() {
        return malUnknownError as prMALError;
    }
    let peak_rec = &mut *peak_rec;

    let num_samples = peak_rec.inNumSampleFrames as usize;
    let importer_data = &*(file_ref as *mut ImporterData);
    
    let num_channels = {
        let ffmpeg = importer_data.ffmpeg.lock().unwrap();
        if !ffmpeg.audio_codec_ctx.is_null() {
            (*ffmpeg.audio_codec_ctx).ch_layout.nb_channels as usize
        } else {
            2
        }
    };

    for ch in 0..num_channels {
        let max_buf = *peak_rec.outMaxima.add(ch);
        let min_buf = *peak_rec.outMinima.add(ch);
        if !max_buf.is_null() && !min_buf.is_null() {
            let max_slice = std::slice::from_raw_parts_mut(max_buf, num_samples);
            let min_slice = std::slice::from_raw_parts_mut(min_buf, num_samples);
            max_slice.fill(0.0);
            min_slice.fill(0.0);
        }
    }

    malNoError as prMALError
}
