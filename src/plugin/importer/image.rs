use std::ffi::c_void;
use std::ptr;
use crate::*;
use crate::ffmpeg_ffi::*;
use crate::importer::types::{ImporterData, CachedFrame, WorkerCommand, CacheState, FFmpegContext};
use std::sync::{Arc, Mutex, Condvar};

pub const AV_NOPTS_VALUE: i64 = i64::MIN;

fn pts_to_frame(pts: i64, start_time: i64, time_base: AVRational, avg_frame_rate: AVRational) -> i32 {
    let pts_offset = if start_time != AV_NOPTS_VALUE {
        pts - start_time
    } else {
        pts
    };
    if avg_frame_rate.num > 0 && avg_frame_rate.den > 0 && time_base.num > 0 && time_base.den > 0 {
        let fps = avg_frame_rate.num as f64 / avg_frame_rate.den as f64;
        let time_sec = pts_offset as f64 * (time_base.num as f64 / time_base.den as f64);
        (time_sec * fps).round() as i32
    } else {
        0
    }
}

/// Background thread worker loop for decoding video frames
pub fn worker_thread_loop(
    ffmpeg: Arc<Mutex<FFmpegContext>>,
    video_stream_idx: i32,
    cache: Arc<(Mutex<CacheState>, Condvar)>,
    rx: std::sync::mpsc::Receiver<WorkerCommand>,
) {
    let mut last_decoded_frame = -999999;
    
    while let Ok(cmd) = rx.recv() {
        match cmd {
            WorkerCommand::DecodeVideoFrame(requested_frame) => {
                // Check if already in cache
                {
                    let guard = cache.0.lock().unwrap();
                    if guard.frame_cache.iter().any(|f| f.frame_number == requested_frame) {
                        cache.1.notify_all();
                        continue;
                    }
                }

                // Lock context to perform seek and decode
                let mut ffmpeg_guard = ffmpeg.lock().unwrap();
                let ffmpeg = &mut *ffmpeg_guard;

                unsafe {
                    let stream = *(*ffmpeg.format_ctx).streams.add(video_stream_idx as usize);
                    let time_base = (*stream).time_base;
                    let avg_frame_rate = (*stream).avg_frame_rate;
                    let start_time = (*stream).start_time;

                    let fps = if avg_frame_rate.num > 0 && avg_frame_rate.den > 0 {
                        avg_frame_rate.num as f64 / avg_frame_rate.den as f64
                    } else {
                        24.0
                    };
                    let time_sec = requested_frame as f64 / fps;
                    let mut base_pts = (time_sec / (time_base.num as f64 / time_base.den as f64)) as i64;
                    if start_time != AV_NOPTS_VALUE {
                        base_pts += start_time;
                    }
                    let target_pts = base_pts;

                    // Only seek if going backwards or jumping forward by more than 10 frames
                    let needs_seek = requested_frame < last_decoded_frame || requested_frame > last_decoded_frame + 10;
                    if needs_seek {
                        av_seek_frame(
                            ffmpeg.format_ctx,
                            video_stream_idx,
                            target_pts,
                            AVSEEK_FLAG_BACKWARD as i32,
                        );
                        avcodec_flush_buffers(ffmpeg.codec_ctx);
                    }

                    let mut decode_attempts = 0;
                    loop {
                        let receive_res = avcodec_receive_frame(ffmpeg.codec_ctx, ffmpeg.frame);
                        if receive_res == 0 {
                            let fmt = (*ffmpeg.frame).format;
                            let is_hw = fmt == AV_PIX_FMT_D3D11
                                || fmt == AV_PIX_FMT_D3D11VA_VLD
                                || fmt == AV_PIX_FMT_DXVA2_VLD
                                || fmt == AV_PIX_FMT_CUDA;
                            
                            let mut sw_frame: *mut AVFrame = ptr::null_mut();
                            let target_frame = if is_hw {
                                sw_frame = av_frame_alloc();
                                if av_hwframe_transfer_data(sw_frame, ffmpeg.frame, 0) == 0 {
                                    (*sw_frame).pts = (*ffmpeg.frame).pts;
                                    (*sw_frame).best_effort_timestamp = (*ffmpeg.frame).best_effort_timestamp;
                                    sw_frame
                                } else {
                                    av_frame_free(&mut sw_frame);
                                    sw_frame = ptr::null_mut();
                                    ffmpeg.frame
                                }
                            } else {
                                ffmpeg.frame
                            };

                            let pts = if (*target_frame).best_effort_timestamp != AV_NOPTS_VALUE {
                                (*target_frame).best_effort_timestamp
                            } else {
                                (*target_frame).pts
                            };
                            
                            let decoded_frame = pts_to_frame(pts, start_time, time_base, avg_frame_rate);

                            if decoded_frame == requested_frame {
                                let cloned = av_frame_clone(target_frame);
                                if !cloned.is_null() {
                                    let mut cache_guard = cache.0.lock().unwrap();
                                    let limit = crate::utils::importer::get_dynamic_cache_size();
                                    if cache_guard.frame_cache.len() >= limit {
                                        let mut oldest = cache_guard.frame_cache.remove(0);
                                        av_frame_free(&mut oldest.frame);
                                    }
                                    cache_guard.frame_cache.push(CachedFrame {
                                        frame_number: requested_frame,
                                        frame: cloned,
                                    });
                                }
                                last_decoded_frame = requested_frame;
                                if !sw_frame.is_null() {
                                    av_frame_free(&mut sw_frame);
                                }
                                break;
                            }
                            if decoded_frame < requested_frame {
                                if !sw_frame.is_null() {
                                    av_frame_free(&mut sw_frame);
                                }
                                continue;
                            }
                            if decoded_frame > requested_frame {
                                // missed frame: cache both the future frame and satisfy the current request using it
                                let cloned = av_frame_clone(target_frame);
                                if !cloned.is_null() {
                                    let mut cache_guard = cache.0.lock().unwrap();
                                    let limit = crate::utils::importer::get_dynamic_cache_size();
                                    if cache_guard.frame_cache.len() >= limit {
                                        let mut oldest = cache_guard.frame_cache.remove(0);
                                        av_frame_free(&mut oldest.frame);
                                    }
                                    cache_guard.frame_cache.push(CachedFrame {
                                        frame_number: decoded_frame,
                                        frame: cloned,
                                    });
                                }
                                let cloned_req = av_frame_clone(target_frame);
                                if !cloned_req.is_null() {
                                    let mut cache_guard = cache.0.lock().unwrap();
                                    cache_guard.frame_cache.push(CachedFrame {
                                        frame_number: requested_frame,
                                        frame: cloned_req,
                                    });
                                }
                                last_decoded_frame = requested_frame;
                                if !sw_frame.is_null() {
                                    av_frame_free(&mut sw_frame);
                                }
                                break;
                            }
                        } else if receive_res == -11 || receive_res == -541478725 {
                            if av_read_frame(ffmpeg.format_ctx, ffmpeg.packet) >= 0 {
                                if (*ffmpeg.packet).stream_index == video_stream_idx {
                                    let send_res = avcodec_send_packet(ffmpeg.codec_ctx, ffmpeg.packet);
                                    av_packet_unref(ffmpeg.packet);
                                    if send_res < 0 {
                                        break;
                                    }
                                } else {
                                    av_packet_unref(ffmpeg.packet);
                                }
                            } else {
                                let send_res = avcodec_send_packet(ffmpeg.codec_ctx, ptr::null_mut());
                                if send_res < 0 {
                                    break;
                                }
                            }
                        } else {
                            break;
                        }

                        decode_attempts += 1;
                        if decode_attempts > 1000 {
                            break;
                        }
                    }
                }

                // Notify waiters
                {
                    let mut cache_guard = cache.0.lock().unwrap();
                    cache_guard.decoding_in_progress.remove(&requested_frame);
                    cache.1.notify_all();
                }
            }
            WorkerCommand::Terminate => break,
        }
    }
}

pub unsafe fn handle_import_image(param1: *mut c_void, param2: *mut c_void) -> prMALError {
    let file_ref = param1;
    if file_ref.is_null() {
        return malUnknownError as prMALError;
    }
    let importer_data = &mut *(file_ref as *mut ImporterData);

    let import_image = param2 as *mut imImportImageRec;
    if import_image.is_null() {
        return malUnknownError as prMALError;
    }
    let import_image = &mut *import_image;

    let requested_frame = import_image.pos;
    let dst_w = import_image.dstWidth;
    let dst_h = import_image.dstHeight;
    let pix_fmt = import_image.pixformat;

    crate::log_debug!("handle_import_image: frame={}, dstSize={}x{}, format={}", requested_frame, dst_w, dst_h, pix_fmt);

    // Lock cache to look up or request decoding
    let mut target_frame: *mut AVFrame = ptr::null_mut();
    {
        let (lock, cvar) = &*importer_data.cache;
        let mut state = lock.lock().unwrap();
        
        for entry in &state.frame_cache {
            if entry.frame_number == requested_frame {
                target_frame = av_frame_clone(entry.frame);
                break;
            }
        }

        if target_frame.is_null() {
            if !state.decoding_in_progress.contains(&requested_frame) {
                state.decoding_in_progress.insert(requested_frame);
                let _ = importer_data.worker_tx.send(WorkerCommand::DecodeVideoFrame(requested_frame));
            }

            while state.decoding_in_progress.contains(&requested_frame) && !state.frame_cache.iter().any(|f| f.frame_number == requested_frame) {
                state = cvar.wait(state).unwrap();
            }

            for entry in &state.frame_cache {
                if entry.frame_number == requested_frame {
                    target_frame = av_frame_clone(entry.frame);
                    break;
                }
            }
        }
    }

    if target_frame.is_null() {
        crate::log_debug!("handle_import_image: failed to decode frame={}", requested_frame);
        return malUnknownError as prMALError;
    }

    // Frame is decoded, now scale/convert it
    let target_pix_fmt = import_image.pixformat;
    let mut return_error = malUnknownError as prMALError;
    
    if target_pix_fmt == PrPixelFormat_PrPixelFormat_BGRA_4444_32f {
        let num_pixels = (import_image.dstWidth * import_image.dstHeight) as usize;
        let mut temp_buf = importer_data.temp_bgra64_buffer.lock().unwrap();
        if temp_buf.len() < num_pixels * 4 {
            temp_buf.resize(num_pixels * 4, 0);
        }
        
        let sws_ctx = sws_getContext(
            (*target_frame).width,
            (*target_frame).height,
            (*target_frame).format,
            import_image.dstWidth,
            import_image.dstHeight,
            AV_PIX_FMT_BGRA64LE,
            SWS_FAST_BILINEAR as i32,
            ptr::null_mut(),
            ptr::null_mut(),
            ptr::null(),
        );
        
        if !sws_ctx.is_null() {
            let row_elements = (import_image.dstWidth * 4) as usize;
            let dest_start = temp_buf.as_mut_ptr().add((import_image.dstHeight - 1) as usize * row_elements);
            let mut dest_data: [*mut u8; 4] = [dest_start as *mut u8, ptr::null_mut(), ptr::null_mut(), ptr::null_mut()];
            let mut dest_linesize: [i32; 4] = [-(import_image.dstWidth * 8) as i32, 0, 0, 0];
            
            sws_scale(
                sws_ctx,
                (*target_frame).data.as_ptr() as *const *const u8,
                (*target_frame).linesize.as_ptr(),
                0,
                (*target_frame).height,
                dest_data.as_mut_ptr(),
                dest_linesize.as_mut_ptr(),
            );
            sws_freeContext(sws_ctx);
            
            let dest_slice = core::slice::from_raw_parts_mut(import_image.pix as *mut f32, num_pixels * 4);
            crate::plugin::shared::bgra64_to_f32(&temp_buf[..num_pixels * 4], dest_slice);
            return_error = malNoError as prMALError;
        }
    } else if target_pix_fmt == PrPixelFormat_PrPixelFormat_BGRA_4444_16u {
        let num_pixels = (import_image.dstWidth * import_image.dstHeight) as usize;
        let mut temp_buf = importer_data.temp_bgra64_buffer.lock().unwrap();
        if temp_buf.len() < num_pixels * 4 {
            temp_buf.resize(num_pixels * 4, 0);
        }
        
        let sws_ctx = sws_getContext(
            (*target_frame).width,
            (*target_frame).height,
            (*target_frame).format,
            import_image.dstWidth,
            import_image.dstHeight,
            AV_PIX_FMT_BGRA64LE,
            SWS_FAST_BILINEAR as i32,
            ptr::null_mut(),
            ptr::null_mut(),
            ptr::null(),
        );
        
        if !sws_ctx.is_null() {
            let row_elements = (import_image.dstWidth * 4) as usize;
            let dest_start = temp_buf.as_mut_ptr().add((import_image.dstHeight - 1) as usize * row_elements);
            let mut dest_data: [*mut u8; 4] = [dest_start as *mut u8, ptr::null_mut(), ptr::null_mut(), ptr::null_mut()];
            let mut dest_linesize: [i32; 4] = [-(import_image.dstWidth * 8) as i32, 0, 0, 0];
            
            sws_scale(
                sws_ctx,
                (*target_frame).data.as_ptr() as *const *const u8,
                (*target_frame).linesize.as_ptr(),
                0,
                (*target_frame).height,
                dest_data.as_mut_ptr(),
                dest_linesize.as_mut_ptr(),
            );
            sws_freeContext(sws_ctx);
            
            let dest_slice = core::slice::from_raw_parts_mut(import_image.pix as *mut u16, num_pixels * 4);
            crate::plugin::shared::bgra64_to_u16_shift(&temp_buf[..num_pixels * 4], dest_slice);
            return_error = malNoError as prMALError;
        }
    } else {
        let sws_ctx = sws_getContext(
            (*target_frame).width,
            (*target_frame).height,
            (*target_frame).format,
            import_image.dstWidth,
            import_image.dstHeight,
            AV_PIX_FMT_BGRA,
            SWS_FAST_BILINEAR as i32,
            ptr::null_mut(),
            ptr::null_mut(),
            ptr::null(),
        );

        if !sws_ctx.is_null() {
            let dest_start = (import_image.pix as *mut u8).add(((import_image.dstHeight - 1) * import_image.rowbytes) as usize);
            let mut dest_data: [*mut u8; 4] = [dest_start, ptr::null_mut(), ptr::null_mut(), ptr::null_mut()];
            let mut dest_linesize: [i32; 4] = [-import_image.rowbytes, 0, 0, 0];

            sws_scale(
                sws_ctx,
                (*target_frame).data.as_ptr() as *const *const u8,
                (*target_frame).linesize.as_ptr(),
                0,
                (*target_frame).height,
                dest_data.as_mut_ptr(),
                dest_linesize.as_mut_ptr(),
            );
            sws_freeContext(sws_ctx);
            
            return_error = malNoError as prMALError;
        }
    }

    av_frame_free(&mut target_frame);
    
    crate::log_debug!("handle_import_image returned: {} for frame={}", return_error, requested_frame);
    
    return_error
}
