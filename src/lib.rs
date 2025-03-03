mod test;

use wasm_bindgen::prelude::*; 
use std::os::raw::{c_char, c_int};

#[link(wasm_import_module = "ffmpeg")]
extern "C" {
    // 自定义的极简帧提取接口
    pub fn extract_frame_to_buffer(
        input: *const c_char,
        time_sec: f64,
        out_buf: *mut u8,
        buf_size: c_int
    ) -> c_int;
}

#[wasm_bindgen]
pub fn get_video_frame(
    input_ptr: *const u8,
    input_len: usize,
    time_sec: f64
) -> Vec<u8> {
    unsafe {
        // 直接在内存中处理视频数据
        let mut buffer = Vec::with_capacity(1024 * 1024); // 预分配 1MB
        let ret = extract_frame_to_buffer(
            input_ptr as *const c_char,
            time_sec,
            buffer.as_mut_ptr(),
            buffer.capacity() as c_int
        );
        
        if ret >= 0 {
            buffer.set_len(ret as usize);
            buffer
        } else {
            Vec::new() // 返回空表示错误
        }
    }
}
