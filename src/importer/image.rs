use std::ffi::c_void;
use std::ptr;
use crate::*;
use crate::ffmpeg_ffi::*;
use crate::importer::types::ImporterData;

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

    let stream = *(*importer_data.format_ctx).streams.add(importer_data.video_stream_idx as usize);
    let time_base = (*stream).time_base;
    let avg_frame_rate = (*stream).avg_frame_rate;
    
    if avg_frame_rate.num > 0 && avg_frame_rate.den > 0 {
        let fps = avg_frame_rate.num as f64 / avg_frame_rate.den as f64;
        let time_sec = requested_frame as f64 / fps;
        let target_pts = (time_sec / (time_base.num as f64 / time_base.den as f64)) as i64;

        av_seek_frame(
            importer_data.format_ctx,
            importer_data.video_stream_idx,
            target_pts,
            AVSEEK_FLAG_BACKWARD as i32,
        );
        avcodec_flush_buffers(importer_data.codec_ctx);
    }

    let mut frame_decoded = false;
    while av_read_frame(importer_data.format_ctx, importer_data.packet) >= 0 {
        if (*importer_data.packet).stream_index == importer_data.video_stream_idx {
            if avcodec_send_packet(importer_data.codec_ctx, importer_data.packet) == 0 {
                if avcodec_receive_frame(importer_data.codec_ctx, importer_data.frame) == 0 {
                    frame_decoded = true;
                    av_packet_unref(importer_data.packet);
                    break;
                }
            }
        }
        av_packet_unref(importer_data.packet);
    }

    if frame_decoded {
        let target_pix_fmt = import_image.pixformat;
        
        if target_pix_fmt == PrPixelFormat_PrPixelFormat_BGRA_4444_32f {
            let num_pixels = (import_image.dstWidth * import_image.dstHeight) as usize;
            let mut temp_bgra64 = vec![0u16; num_pixels * 4];
            
            let sws_ctx = sws_getContext(
                (*importer_data.frame).width,
                (*importer_data.frame).height,
                (*importer_data.frame).format,
                import_image.dstWidth,
                import_image.dstHeight,
                AV_PIX_FMT_BGRA64LE,
                SWS_BILINEAR as i32,
                ptr::null_mut(),
                ptr::null_mut(),
                ptr::null(),
            );
            
            if !sws_ctx.is_null() {
                let row_elements = (import_image.dstWidth * 4) as usize;
                let dest_start = temp_bgra64.as_mut_ptr().add((import_image.dstHeight - 1) as usize * row_elements);
                let mut dest_data: [*mut u8; 4] = [dest_start as *mut u8, ptr::null_mut(), ptr::null_mut(), ptr::null_mut()];
                let mut dest_linesize: [i32; 4] = [-(import_image.dstWidth * 8) as i32, 0, 0, 0];
                
                sws_scale(
                    sws_ctx,
                    (*importer_data.frame).data.as_ptr() as *const *const u8,
                    (*importer_data.frame).linesize.as_ptr(),
                    0,
                    (*importer_data.frame).height,
                    dest_data.as_mut_ptr(),
                    dest_linesize.as_mut_ptr(),
                );
                sws_freeContext(sws_ctx);
                
                let dest_slice = core::slice::from_raw_parts_mut(import_image.pix as *mut f32, num_pixels * 4);
                for (s, d) in temp_bgra64.iter().zip(dest_slice.iter_mut()) {
                    *d = *s as f32 / 65535.0;
                }
                return malNoError as prMALError;
            }
        } else if target_pix_fmt == PrPixelFormat_PrPixelFormat_BGRA_4444_16u {
            let num_pixels = (import_image.dstWidth * import_image.dstHeight) as usize;
            let mut temp_bgra64 = vec![0u16; num_pixels * 4];
            
            let sws_ctx = sws_getContext(
                (*importer_data.frame).width,
                (*importer_data.frame).height,
                (*importer_data.frame).format,
                import_image.dstWidth,
                import_image.dstHeight,
                AV_PIX_FMT_BGRA64LE,
                SWS_BILINEAR as i32,
                ptr::null_mut(),
                ptr::null_mut(),
                ptr::null(),
            );
            
            if !sws_ctx.is_null() {
                let row_elements = (import_image.dstWidth * 4) as usize;
                let dest_start = temp_bgra64.as_mut_ptr().add((import_image.dstHeight - 1) as usize * row_elements);
                let mut dest_data: [*mut u8; 4] = [dest_start as *mut u8, ptr::null_mut(), ptr::null_mut(), ptr::null_mut()];
                let mut dest_linesize: [i32; 4] = [-(import_image.dstWidth * 8) as i32, 0, 0, 0];
                
                sws_scale(
                    sws_ctx,
                    (*importer_data.frame).data.as_ptr() as *const *const u8,
                    (*importer_data.frame).linesize.as_ptr(),
                    0,
                    (*importer_data.frame).height,
                    dest_data.as_mut_ptr(),
                    dest_linesize.as_mut_ptr(),
                );
                sws_freeContext(sws_ctx);
                
                let dest_slice = core::slice::from_raw_parts_mut(import_image.pix as *mut u16, num_pixels * 4);
                for (s, d) in temp_bgra64.iter().zip(dest_slice.iter_mut()) {
                    *d = *s >> 1;
                }
                return malNoError as prMALError;
            }
        } else {
            let sws_ctx = sws_getContext(
                (*importer_data.frame).width,
                (*importer_data.frame).height,
                (*importer_data.frame).format,
                import_image.dstWidth,
                import_image.dstHeight,
                AV_PIX_FMT_BGRA,
                SWS_BILINEAR as i32,
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
                    (*importer_data.frame).data.as_ptr() as *const *const u8,
                    (*importer_data.frame).linesize.as_ptr(),
                    0,
                    (*importer_data.frame).height,
                    dest_data.as_mut_ptr(),
                    dest_linesize.as_mut_ptr(),
                );
                sws_freeContext(sws_ctx);
                
                return malNoError as prMALError;
            }
        }
    }
    
    malUnknownError as prMALError
}
