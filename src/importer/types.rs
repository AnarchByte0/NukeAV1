use crate::ffmpeg_ffi::*;

pub struct ImporterData {
    pub format_ctx: *mut AVFormatContext,
    pub codec_ctx: *mut AVCodecContext,
    pub video_stream_idx: i32,
    pub frame: *mut AVFrame,
    pub packet: *mut AVPacket,

    // Audio support fields
    pub audio_stream_idx: i32,
    pub audio_codec_ctx: *mut AVCodecContext,
    pub audio_frame: *mut AVFrame,
    pub swr_ctx: *mut SwrContext,
    pub audio_buffer: Vec<Vec<f32>>,
    pub audio_buffer_start_sample: i64,
    pub needs_first_pts: bool,
    pub hw_device_ctx: *mut AVBufferRef,
}

impl Drop for ImporterData {
    fn drop(&mut self) {
        unsafe {
            if !self.packet.is_null() {
                av_packet_free(&mut self.packet);
            }
            if !self.frame.is_null() {
                av_frame_free(&mut self.frame);
            }
            if !self.codec_ctx.is_null() {
                avcodec_free_context(&mut self.codec_ctx);
            }
            if !self.audio_frame.is_null() {
                av_frame_free(&mut self.audio_frame);
            }
            if !self.audio_codec_ctx.is_null() {
                avcodec_free_context(&mut self.audio_codec_ctx);
            }
            if !self.swr_ctx.is_null() {
                swr_free(&mut self.swr_ctx);
            }
            if !self.hw_device_ctx.is_null() {
                av_buffer_unref(&mut self.hw_device_ctx);
            }
            if !self.format_ctx.is_null() {
                avformat_close_input(&mut self.format_ctx);
            }
        }
    }
}
