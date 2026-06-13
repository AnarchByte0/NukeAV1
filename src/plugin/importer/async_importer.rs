use std::ffi::c_void;
use std::os::raw::c_int;
use std::ptr;

use crate::*;
use crate::importer::types::{ImporterData, WorkerCommand};

// ─────────────────────────────────────────────────────────────────────────────
// Public structures
// ─────────────────────────────────────────────────────────────────────────────

pub struct AsyncImporterData {
    /// Raw address of the ImporterData (file private data).
    pub importer_data_ptr: usize,
    /// piSuites pointer (stored as usize so we can send across threads if needed).
    pub pi_suites: usize,
    /// Video width, height, fps captured at open time.
    pub vid_width: i32,
    pub vid_height: i32,
    pub fps: f64,
}

unsafe impl Send for AsyncImporterData {}
unsafe impl Sync for AsyncImporterData {}

// ─────────────────────────────────────────────────────────────────────────────
// imCreateAsyncImporter
// ─────────────────────────────────────────────────────────────────────────────

pub unsafe fn handle_create_async_importer(
    std_parms: *mut crate::imStdParms,
    param1: *mut c_void,
    _param2: *mut c_void,
) -> prMALError {
    let creation_rec = param1 as *mut imAsyncImporterCreationRec;
    if creation_rec.is_null() {
        crate::log_debug!("handle_create_async_importer failed: creation_rec is null");
        return malUnknownError as prMALError;
    }

    let file_ref = (*creation_rec).inPrivateData; // ImporterData*
    if file_ref.is_null() {
        crate::log_debug!("handle_create_async_importer failed: file_ref is null");
        return malUnknownError as prMALError;
    }

    crate::log_debug!("handle_create_async_importer called, file_ref: {:?}", file_ref);

    let importer_data = &*(file_ref as *mut ImporterData);

    let (vid_width, vid_height, fps) = {
        let ffmpeg = importer_data.ffmpeg.lock().unwrap();
        let codec_ctx = ffmpeg.codec_ctx;
        let stream = *(*ffmpeg.format_ctx)
            .streams
            .add(importer_data.video_stream_idx as usize);
        let w = if !codec_ctx.is_null() { (*codec_ctx).width  } else { 1920 };
        let h = if !codec_ctx.is_null() { (*codec_ctx).height } else { 1080 };
        let rate = (*stream).avg_frame_rate;
        let f = if rate.den > 0 && rate.num > 0 {
            rate.num as f64 / rate.den as f64
        } else {
            24.0
        };
        (w, h, f)
    };

    let pi_suites: usize = if !std_parms.is_null() {
        (*std_parms).piSuites as usize
    } else {
        0
    };

    let async_data = Box::new(AsyncImporterData {
        importer_data_ptr: file_ref as usize,
        pi_suites,
        vid_width,
        vid_height,
        fps,
    });
    let async_data_raw = Box::into_raw(async_data);

    // Store back-pointer so aiGetFrame can reach AsyncImporterData
    // from imSourceVideoRec.inPrivateData (which is ImporterData).
    (&mut *(file_ref as *mut ImporterData)).async_data_ptr = async_data_raw as *mut c_void;

    (*creation_rec).outAsyncEntry = Some(async_importer_entry);
    (*creation_rec).outAsyncPrivateData = async_data_raw as *mut c_void;

    crate::log_debug!("handle_create_async_importer succeeded, async_data_raw: {:?}", async_data_raw);

    malNoError as prMALError
}

// ─────────────────────────────────────────────────────────────────────────────
// Async importer entry point (called by Premiere Pro)
// ─────────────────────────────────────────────────────────────────────────────

pub unsafe extern "C" fn async_importer_entry(selector: c_int, param: *mut c_void) -> prMALError {
    crate::log_debug!("async_importer_entry called: selector = {}, param = {:?}", selector, param);

    // aiInitiateAsyncRead — queue the request and return immediately.
    if selector == aiInitiateAsyncRead as c_int {
        let request = param as *mut aiAsyncRequest;
        if !request.is_null() {
            let async_data_raw = (*request).inPrivateData as *mut AsyncImporterData;
            if !async_data_raw.is_null() {
                let async_data = &*async_data_raw;
                if async_data.importer_data_ptr != 0 {
                    let importer_data = &*(async_data.importer_data_ptr as *const ImporterData);
                    
                    let frame_time = (*request).inSourceRec.inFrameTime;
                    const TICKS_PER_SEC: f64 = 254016000000.0;
                    let time_sec = frame_time as f64 / TICKS_PER_SEC;
                    let frame_number = (time_sec * async_data.fps).round() as i32;
                    
                    let (lock, _cvar) = &*importer_data.cache;
                    let mut state = lock.lock().unwrap();
                    if !state.frame_cache.iter().any(|f| f.frame_number == frame_number) 
                        && !state.decoding_in_progress.contains(&frame_number) 
                    {
                        state.decoding_in_progress.insert(frame_number);
                        let _ = importer_data.worker_tx.send(WorkerCommand::DecodeVideoFrame(frame_number));
                    }
                }
            }
        }
        return aiNoError as prMALError;
    }

    // aiCancelAsyncRead — hint only, nothing to do.
    if selector == aiCancelAsyncRead as c_int {
        return aiNoError as prMALError;
    }

    // aiFlush — cancel all pending reads (none in our synchronous model).
    if selector == aiFlush as c_int {
        return aiNoError as prMALError;
    }

    // aiGetFrame — actually decode and deliver the frame.
    if selector == aiGetFrame as c_int {
        let res = handle_ai_get_frame(param);
        crate::log_debug!("async_importer_entry: aiGetFrame returned {}", res);
        return res;
    }

    // aiClose — free AsyncImporterData.
    if selector == aiClose as c_int {
        let async_data_ptr = param as *mut AsyncImporterData;
        if !async_data_ptr.is_null() {
            let ad = &*async_data_ptr;
            // Null out back-pointer in ImporterData.
            if ad.importer_data_ptr != 0 {
                let imp = &mut *(ad.importer_data_ptr as *mut ImporterData);
                imp.async_data_ptr = ptr::null_mut();
            }
            drop(Box::from_raw(async_data_ptr));
        }
        return aiNoError as prMALError;
    }

    aiUnsupported as prMALError
}

// ─────────────────────────────────────────────────────────────────────────────
// Core frame delivery
// ─────────────────────────────────────────────────────────────────────────────

unsafe fn handle_ai_get_frame(param: *mut c_void) -> prMALError {
    let src_video = param as *mut imSourceVideoRec;
    if src_video.is_null() {
        crate::log_debug!("handle_ai_get_frame: src_video is null");
        return aiUnknownError as prMALError;
    }

    let private_data = (*src_video).inPrivateData;
    if private_data.is_null() {
        crate::log_debug!("handle_ai_get_frame: inPrivateData is null");
        return aiFrameNotFound as prMALError;
    }

    // In aiGetFrame, the inPrivateData pointer is our outAsyncPrivateData (AsyncImporterData*).
    let async_data = &*(private_data as *mut AsyncImporterData);

    if async_data.importer_data_ptr == 0 {
        crate::log_debug!("handle_ai_get_frame: importer_data_ptr is 0!");
        return aiFrameNotFound as prMALError;
    }
    let importer_data = &mut *(async_data.importer_data_ptr as *mut ImporterData);

    crate::log_debug!(
        "handle_ai_get_frame: async_data={:?}, importer_data={:?}",
        private_data,
        async_data.importer_data_ptr as *mut ImporterData
    );

    // ── Determine what Premiere wants ────────────────────────────────────────

    let frame_time    = (*src_video).inFrameTime;
    let num_formats   = (*src_video).inNumFrameFormats;
    let formats_ptr   = (*src_video).inFrameFormats;

    // Convert PrTime to a frame number (PrTime ticks = 254016000000 per second).
    const TICKS_PER_SEC: f64 = 254016000000.0;
    let time_sec      = frame_time as f64 / TICKS_PER_SEC;
    let frame_number  = (time_sec * async_data.fps).round() as i32;

    crate::log_debug!(
        "handle_ai_get_frame: frame_time={}, frame_number={}, fps={}",
        frame_time,
        frame_number,
        async_data.fps
    );

    // Choose pixel format & size — prefer BGRA_4444_32f (HDR float), fall back to 8u.
    let mut chosen_fmt = PrPixelFormat_PrPixelFormat_BGRA_4444_8u;
    let mut chosen_width = async_data.vid_width;
    let mut chosen_height = async_data.vid_height;
    if num_formats > 0 && !formats_ptr.is_null() {
        let mut found_index = 0;
        for i in 0..(num_formats as usize) {
            let fmt = (*formats_ptr.add(i)).inPixelFormat;
            if fmt == PrPixelFormat_PrPixelFormat_BGRA_4444_32f {
                chosen_fmt = fmt;
                found_index = i;
                break;
            }
            if fmt == PrPixelFormat_PrPixelFormat_BGRA_4444_8u {
                chosen_fmt = fmt;
                found_index = i;
            }
        }
        let req_w = (*formats_ptr.add(found_index)).inFrameWidth;
        let req_h = (*formats_ptr.add(found_index)).inFrameHeight;
        if req_w > 0 && req_h > 0 {
            chosen_width = req_w;
            chosen_height = req_h;
        }
    }

    let width  = chosen_width;
    let height = chosen_height;

    // ── Acquire SPBasicSuite ─────────────────────────────────────────────────

    let pi_suites = async_data.pi_suites as *mut piSuites;
    if pi_suites.is_null() {
        crate::log_debug!("handle_ai_get_frame: pi_suites is null");
        return aiFrameNotFound as prMALError;
    }

    let util_funcs = (*pi_suites).utilFuncs;
    if util_funcs.is_null() { return aiFrameNotFound as prMALError; }

    let get_sp_fn = match (*util_funcs).getSPBasicSuite {
        Some(f) => f,
        None    => return aiFrameNotFound as prMALError,
    };
    let sp_basic: *mut SPBasicSuite = get_sp_fn();
    if sp_basic.is_null() { return aiFrameNotFound as prMALError; }

    // ── Acquire PPixCreator Suite (version 1 — simplest, always present) ─────

    let suite_name_cstr = b"Premiere PPix Creator Suite\0";
    let mut creator_ptr: *const c_void = ptr::null();
    let acquire_fn = match (*sp_basic).AcquireSuite {
        Some(f) => f,
        None    => return aiFrameNotFound as prMALError,
    };

    let err = acquire_fn(
        suite_name_cstr.as_ptr() as *const i8,
        1, // kPrSDKPPixCreatorSuiteVersion = 1
        &mut creator_ptr as *mut *const c_void,
    );
    if err != 0 || creator_ptr.is_null() {
        return aiFrameNotFound as prMALError;
    }
    let ppix_creator = &*(creator_ptr as *const PrSDKPPixCreatorSuite);

    // Helper closure to release the suite before returning.
    let release_suite = || {
        if let Some(release) = (*sp_basic).ReleaseSuite {
            release(suite_name_cstr.as_ptr() as *const i8, 1);
        }
    };

    // ── Create a new PPix with the correct format and size ───────────────────

    #[cfg(target_os = "windows")]
    let bounds = prRect {
        top:    0,
        left:   0,
        bottom: height,
        right:  width,
    };
    #[cfg(not(target_os = "windows"))]
    let bounds = prRect {
        top:    0,
        left:   0,
        bottom: height as i16,
        right:  width as i16,
    };

    let create_ppix_fn = match ppix_creator.CreatePPix {
        Some(f) => f,
        None    => { release_suite(); return aiFrameNotFound as prMALError; }
    };

    let mut ppix_hand: PPixHand = ptr::null_mut();
    let suite_err = create_ppix_fn(
        &mut ppix_hand,
        PrPPixBufferAccess_PrPPixBufferAccess_ReadWrite,
        chosen_fmt,
        &bounds as *const prRect,
    );
    if suite_err != 0 || ppix_hand.is_null() {
        release_suite();
        return aiFrameNotFound as prMALError;
    }

    // ── Get pixel buffer pointer and row-bytes from the new PPix ─────────────
    // We use the modern PrSDKPPixSuite for this.

    let ppix_suite_name = b"Premiere PPix Suite\0";
    let mut ppix_suite_ptr: *const c_void = ptr::null();
    let err2 = acquire_fn(
        ppix_suite_name.as_ptr() as *const i8,
        1, // kPrSDKPPixSuiteVersion = 1
        &mut ppix_suite_ptr as *mut *const c_void,
    );

    if err2 != 0 || ppix_suite_ptr.is_null() {
        // Fall back to legacy ppixFuncs.
        let ppix_funcs = (*pi_suites).ppixFuncs;
        if ppix_funcs.is_null() {
            release_suite();
            return aiFrameNotFound as prMALError;
        }
        let get_pixels = match (*ppix_funcs).ppixGetPixels {
            Some(f) => f,
            None    => { release_suite(); return aiFrameNotFound as prMALError; }
        };
        let get_rb = match (*ppix_funcs).ppixGetRowbytes {
            Some(f) => f,
            None    => { release_suite(); return aiFrameNotFound as prMALError; }
        };
        let dst = get_pixels(ppix_hand);
        let rowbytes = get_rb(ppix_hand);

        let result = do_decode(importer_data, frame_number, width, height, rowbytes, chosen_fmt, dst as *mut i8);
        release_suite();
        if result != malNoError as prMALError {
            if let Some(dispose) = (*ppix_funcs).ppixDispose {
                dispose(ppix_hand);
            }
            return aiFrameNotFound as prMALError;
        }
        if let Some(out) = (*src_video).outFrame.as_mut() {
            *out = ppix_hand;
        }
        return aiNoError as prMALError;
    }

    // Modern path via PrSDKPPixSuite.
    let ppix_suite = &*(ppix_suite_ptr as *const PrSDKPPixSuite);

    let mut dst_ptr: *mut i8 = ptr::null_mut();
    let get_px_err = match ppix_suite.GetPixels {
        Some(f) => f(ppix_hand, PrPPixBufferAccess_PrPPixBufferAccess_ReadWrite, &mut dst_ptr),
        None    => { release_suite(); return aiFrameNotFound as prMALError; }
    };

    let mut rowbytes: i32 = 0;
    let _rb_err = match ppix_suite.GetRowBytes {
        Some(f) => f(ppix_hand, &mut rowbytes),
        None    => 0,
    };

    if let Some(rel) = (*sp_basic).ReleaseSuite {
        rel(ppix_suite_name.as_ptr() as *const i8, 1);
    }

    if get_px_err != 0 || dst_ptr.is_null() || rowbytes <= 0 {
        if let Some(dispose) = ppix_suite.Dispose {
            dispose(ppix_hand);
        }
        release_suite();
        return aiFrameNotFound as prMALError;
    }

    // ── Decode directly into the PPix pixel buffer ───────────────────────────
    let decode_result = do_decode(importer_data, frame_number, width, height, rowbytes, chosen_fmt, dst_ptr);

    release_suite();

    if decode_result != malNoError as prMALError {
        if let Some(dispose) = ppix_suite.Dispose {
            dispose(ppix_hand);
        }
        return aiFrameNotFound as prMALError;
    }

    // ── Hand PPix to Premiere ─────────────────────────────────────────────────
    if (*src_video).outFrame.is_null() {
        if let Some(dispose) = ppix_suite.Dispose {
            dispose(ppix_hand);
        }
        return aiUnknownError as prMALError;
    }
    *(*src_video).outFrame = ppix_hand;

    aiNoError as prMALError
}

// ─────────────────────────────────────────────────────────────────────────────
// Reuse existing synchronous decode path from image.rs
// ─────────────────────────────────────────────────────────────────────────────

unsafe fn do_decode(
    importer_data: *mut ImporterData,
    frame_number: i32,
    width: i32,
    height: i32,
    rowbytes: i32,
    pix_fmt: PrPixelFormat,
    dst: *mut i8,
) -> prMALError {
    let mut fake_rec: imImportImageRec = std::mem::zeroed();
    fake_rec.pos        = frame_number;
    fake_rec.dstWidth   = width;
    fake_rec.dstHeight  = height;
    fake_rec.rowbytes   = rowbytes;
    fake_rec.pixformat  = pix_fmt;
    fake_rec.pix        = dst;
    fake_rec.privatedata = importer_data as *mut c_void;

    crate::importer::image::handle_import_image(
        importer_data as *mut c_void,
        &mut fake_rec as *mut _ as *mut c_void,
    )
}

pub unsafe fn handle_get_source_video(param1: *mut c_void, param2: *mut c_void) -> prMALError {
    let importer_data = param1 as *mut ImporterData;
    let src_video = param2 as *mut imSourceVideoRec;
    if importer_data.is_null() || src_video.is_null() {
        return malUnknownError as prMALError;
    }
    
    let frame_time = (*src_video).inFrameTime;
    let num_formats = (*src_video).inNumFrameFormats;
    let formats_ptr = (*src_video).inFrameFormats;
    
    let fps = {
        let ffmpeg = (*importer_data).ffmpeg.lock().unwrap();
        let stream = *(*ffmpeg.format_ctx).streams.add((*importer_data).video_stream_idx as usize);
        let rate = (*stream).avg_frame_rate;
        if rate.den > 0 && rate.num > 0 {
            rate.num as f64 / rate.den as f64
        } else {
            24.0
        }
    };
    
    const TICKS_PER_SEC: f64 = 254016000000.0;
    let time_sec = frame_time as f64 / TICKS_PER_SEC;
    let frame_number = (time_sec * fps).round() as i32;
    
    let mut chosen_fmt = PrPixelFormat_PrPixelFormat_BGRA_4444_8u;
    let mut chosen_width = 1920;
    let mut chosen_height = 1080;
    {
        let ffmpeg = (*importer_data).ffmpeg.lock().unwrap();
        if !ffmpeg.codec_ctx.is_null() {
            chosen_width = (*ffmpeg.codec_ctx).width;
            chosen_height = (*ffmpeg.codec_ctx).height;
        }
    }
    
    if num_formats > 0 && !formats_ptr.is_null() {
        let mut found_index = 0;
        for i in 0..(num_formats as usize) {
            let fmt = (*formats_ptr.add(i)).inPixelFormat;
            if fmt == PrPixelFormat_PrPixelFormat_BGRA_4444_32f {
                chosen_fmt = fmt;
                found_index = i;
                break;
            }
            if fmt == PrPixelFormat_PrPixelFormat_BGRA_4444_8u {
                chosen_fmt = fmt;
                found_index = i;
            }
        }
        let req_w = (*formats_ptr.add(found_index)).inFrameWidth;
        let req_h = (*formats_ptr.add(found_index)).inFrameHeight;
        if req_w > 0 && req_h > 0 {
            chosen_width = req_w;
            chosen_height = req_h;
        }
    }
    
    let width = chosen_width;
    let height = chosen_height;
    
    let std_parms = (*importer_data).std_parms;
    if std_parms.is_null() { return malUnknownError as prMALError; }
    
    let sp_basic = (*std_parms).piSuites as *mut piSuites;
    if sp_basic.is_null() { return malUnknownError as prMALError; }
    
    let util_funcs = (*sp_basic).utilFuncs;
    if util_funcs.is_null() { return malUnknownError as prMALError; }
    
    let get_sp_fn = match (*util_funcs).getSPBasicSuite {
        Some(f) => f,
        None    => return malUnknownError as prMALError,
    };
    let sp_basic_suite: *mut SPBasicSuite = get_sp_fn();
    if sp_basic_suite.is_null() { return malUnknownError as prMALError; }
    
    let suite_name_cstr = b"Premiere PPix Creator Suite\0";
    let mut creator_ptr: *const c_void = ptr::null();
    let acquire_fn = match (*sp_basic_suite).AcquireSuite {
        Some(f) => f,
        None    => return malUnknownError as prMALError,
    };
    
    let err = acquire_fn(
        suite_name_cstr.as_ptr() as *const i8,
        1,
        &mut creator_ptr as *mut *const c_void,
    );
    if err != 0 || creator_ptr.is_null() {
        return malUnknownError as prMALError;
    }
    let ppix_creator = &*(creator_ptr as *const PrSDKPPixCreatorSuite);
    
    let release_suite = || {
        if let Some(release) = (*sp_basic_suite).ReleaseSuite {
            release(suite_name_cstr.as_ptr() as *const i8, 1);
        }
    };
    
    #[cfg(target_os = "windows")]
    let bounds = prRect {
        top: 0,
        left: 0,
        bottom: height,
        right: width,
    };
    #[cfg(not(target_os = "windows"))]
    let bounds = prRect {
        top: 0,
        left: 0,
        bottom: height as i16,
        right: width as i16,
    };
    
    let create_ppix_fn = match ppix_creator.CreatePPix {
        Some(f) => f,
        None    => { release_suite(); return malUnknownError as prMALError; }
    };
    
    let mut ppix_hand: PPixHand = ptr::null_mut();
    let suite_err = create_ppix_fn(
        &mut ppix_hand,
        PrPPixBufferAccess_PrPPixBufferAccess_ReadWrite,
        chosen_fmt,
        &bounds as *const prRect,
    );
    if suite_err != 0 || ppix_hand.is_null() {
        release_suite();
        return malUnknownError as prMALError;
    }
    
    let ppix_suite_name = b"Premiere PPix Suite\0";
    let mut ppix_suite_ptr: *const c_void = ptr::null();
    let err2 = acquire_fn(
        ppix_suite_name.as_ptr() as *const i8,
        1,
        &mut ppix_suite_ptr as *mut *const c_void,
    );
    
    if err2 != 0 || ppix_suite_ptr.is_null() {
        let ppix_funcs = (*sp_basic).ppixFuncs;
        if ppix_funcs.is_null() {
            release_suite();
            return malUnknownError as prMALError;
        }
        let get_pixels = match (*ppix_funcs).ppixGetPixels {
            Some(f) => f,
            None    => { release_suite(); return malUnknownError as prMALError; }
        };
        let get_rb = match (*ppix_funcs).ppixGetRowbytes {
            Some(f) => f,
            None    => { release_suite(); return malUnknownError as prMALError; }
        };
        let dst = get_pixels(ppix_hand);
        let rowbytes = get_rb(ppix_hand);
        
        let result = do_decode(importer_data, frame_number, width, height, rowbytes, chosen_fmt, dst as *mut i8);
        release_suite();
        if result != malNoError as prMALError {
            if let Some(dispose) = (*ppix_funcs).ppixDispose {
                dispose(ppix_hand);
            }
            return malUnknownError as prMALError;
        }
        if let Some(out) = (*src_video).outFrame.as_mut() {
            *out = ppix_hand;
        }
        return malNoError as prMALError;
    }
    
    let ppix_suite = &*(ppix_suite_ptr as *const PrSDKPPixSuite);
    let mut dst_ptr: *mut i8 = ptr::null_mut();
    let get_px_err = match ppix_suite.GetPixels {
        Some(f) => f(ppix_hand, PrPPixBufferAccess_PrPPixBufferAccess_ReadWrite, &mut dst_ptr),
        None    => { release_suite(); return malUnknownError as prMALError; }
    };
    
    let mut rowbytes: i32 = 0;
    let _rb_err = match ppix_suite.GetRowBytes {
        Some(f) => f(ppix_hand, &mut rowbytes),
        None    => 0,
    };
    
    if let Some(rel) = (*sp_basic_suite).ReleaseSuite {
        rel(ppix_suite_name.as_ptr() as *const i8, 1);
    }
    
    if get_px_err != 0 || dst_ptr.is_null() || rowbytes <= 0 {
        if let Some(dispose) = ppix_suite.Dispose {
            dispose(ppix_hand);
        }
        release_suite();
        return malUnknownError as prMALError;
    }
    
    let decode_result = do_decode(importer_data, frame_number, width, height, rowbytes, chosen_fmt, dst_ptr);
    release_suite();
    
    if decode_result != malNoError as prMALError {
        if let Some(dispose) = ppix_suite.Dispose {
            dispose(ppix_hand);
        }
        return malUnknownError as prMALError;
    }
    
    if (*src_video).outFrame.is_null() {
        if let Some(dispose) = ppix_suite.Dispose {
            dispose(ppix_hand);
        }
        return malUnknownError as prMALError;
    }
    *(*src_video).outFrame = ppix_hand;
    
    malNoError as prMALError
}
