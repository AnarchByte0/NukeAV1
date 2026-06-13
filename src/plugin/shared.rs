/// Convert BGRA64 buffer to f32 (normalized 0.0 to 1.0)
#[inline]
pub fn bgra64_to_f32(src: &[u16], dst: &mut [f32]) {
    let len = src.len().min(dst.len());
    for i in 0..len {
        dst[i] = src[i] as f32 / 65535.0;
    }
}

/// Convert f32 buffer (normalized 0.0 to 1.0) to BGRA64 (0 to 65535)
#[inline]
pub fn f32_to_bgra64(src: &[f32], dst: &mut [u16]) {
    let len = src.len().min(dst.len());
    for i in 0..len {
        dst[i] = (src[i] * 65535.0).clamp(0.0, 65535.0) as u16;
    }
}

/// Convert BGRA64 buffer to u16 buffer (shifted by 1 bit for 15-bit Adobe format)
#[inline]
pub fn bgra64_to_u16_shift(src: &[u16], dst: &mut [u16]) {
    let len = src.len().min(dst.len());
    for i in 0..len {
        dst[i] = src[i] >> 1;
    }
}
