use crate::ffmpeg_ffi::*;
use std::sync::{Arc, Mutex, Condvar};
use std::sync::mpsc::Sender;
use std::collections::HashSet;
use std::thread::JoinHandle;

#[derive(Clone)]
pub struct CachedFrame {
    pub frame_number: i32,
    pub frame: *mut AVFrame,
}
unsafe impl Send for CachedFrame {}
unsafe impl Sync for CachedFrame {}

pub struct FFmpegContext {
    pub format_ctx: *mut AVFormatContext,
    pub codec_ctx: *mut AVCodecContext,
    pub frame: *mut AVFrame,
    pub packet: *mut AVPacket,
    pub audio_codec_ctx: *mut AVCodecContext,
    pub audio_frame: *mut AVFrame,
    pub swr_ctx: *mut SwrContext,
}
unsafe impl Send for FFmpegContext {}
unsafe impl Sync for FFmpegContext {}

pub struct CacheState {
    pub frame_cache: Vec<CachedFrame>,
    pub decoding_in_progress: HashSet<i32>,
}

pub enum WorkerCommand {
    DecodeVideoFrame(i32),
    Terminate,
}

pub struct ImporterData {
    pub ffmpeg: Arc<Mutex<FFmpegContext>>,
    pub video_stream_idx: i32,
    pub audio_stream_idx: i32,
    
    // Threading and Cache
    pub cache: Arc<(Mutex<CacheState>, Condvar)>,
    pub worker_tx: Sender<WorkerCommand>,
    pub worker_thread: Option<JoinHandle<()>>,

    pub audio_buffer: Vec<Vec<f32>>,
    pub audio_buffer_start_sample: i64,
    pub needs_first_pts: bool,
    pub hw_device_ctx: *mut AVBufferRef,
    pub last_decoded_frame: Mutex<i32>,
    pub temp_bgra64_buffer: Mutex<Vec<u16>>,
    pub std_parms: *mut crate::imStdParms,
    pub async_data_ptr: *mut std::ffi::c_void,
}

impl Drop for ImporterData {
    fn drop(&mut self) {
        // Stop background thread
        let _ = self.worker_tx.send(WorkerCommand::Terminate);
        if let Some(handle) = self.worker_thread.take() {
            let _ = handle.join();
        }

        unsafe {
            // Free the cache frames
            if let Ok(guard) = self.cache.0.lock() {
                for f in &guard.frame_cache {
                    let mut frame_ptr = f.frame;
                    if !frame_ptr.is_null() {
                        av_frame_free(&mut frame_ptr);
                    }
                }
            }

            if !self.hw_device_ctx.is_null() {
                av_buffer_unref(&mut self.hw_device_ctx);
            }

            // Free context struct itself
            if let Ok(mut ffmpeg) = self.ffmpeg.lock() {
                if !ffmpeg.packet.is_null() {
                    av_packet_free(&mut ffmpeg.packet);
                }
                if !ffmpeg.frame.is_null() {
                    av_frame_free(&mut ffmpeg.frame);
                }
                if !ffmpeg.codec_ctx.is_null() {
                    avcodec_free_context(&mut ffmpeg.codec_ctx);
                }
                if !ffmpeg.audio_frame.is_null() {
                    av_frame_free(&mut ffmpeg.audio_frame);
                }
                if !ffmpeg.audio_codec_ctx.is_null() {
                    avcodec_free_context(&mut ffmpeg.audio_codec_ctx);
                }
                if !ffmpeg.swr_ctx.is_null() {
                    swr_free(&mut ffmpeg.swr_ctx);
                }
                if !ffmpeg.format_ctx.is_null() {
                    avformat_close_input(&mut ffmpeg.format_ctx);
                }
            }
        }
    }
}
