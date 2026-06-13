pub mod importer;
pub mod exporter;

pub use exporter::UIBuilder;

use crate::prUTF16Char;
use std::ffi::CString;
use std::os::raw::c_char;
use std::ptr;

/// Copy a Rust string slice into a null-terminated raw C char buffer
pub fn copy_c_str(src: &str, dst: &mut [c_char]) {
    let c_str = CString::new(src).unwrap();
    let bytes = c_str.as_bytes_with_nul();
    let len = std::cmp::min(bytes.len(), dst.len());
    unsafe {
        ptr::copy_nonoverlapping(bytes.as_ptr() as *const c_char, dst.as_mut_ptr(), len);
        if len < dst.len() {
            dst[len] = 0;
        } else if dst.len() > 0 {
            dst[dst.len() - 1] = 0;
        }
    }
}

/// Convert a null-terminated UTF-16 pointer into a Rust String
pub unsafe fn get_utf16_string(ptr: *const prUTF16Char) -> String {
    let mut len = 0;
    while *ptr.add(len) != 0 {
        len += 1;
    }
    let slice = std::slice::from_raw_parts(ptr, len);
    String::from_utf16_lossy(slice)
}

/// Copy a Rust string slice into a fixed-size Adobe UTF-16 buffer
pub unsafe fn str_to_utf16(s: &str, out: *mut prUTF16Char, max_len: usize) {
    let mut i = 0;
    for c in s.encode_utf16() {
        if i >= max_len - 1 {
            break;
        }
        *out.add(i) = c;
        i += 1;
    }
    *out.add(i) = 0; // null-terminator
}

/// Allocate a persistent UTF-16 string on the heap and return a raw pointer
pub fn leak_utf16(s: &str) -> *const prUTF16Char {
    let mut vec: Vec<prUTF16Char> = s.encode_utf16().collect();
    vec.push(0); // null-terminator
    Box::leak(vec.into_boxed_slice()).as_ptr()
}
