// wasm_interface.rs
// 提供WASM接口，处理与JavaScript的交互

use crate::error::{log_error, VideoResult};
use crate::video_processor;
use std::slice;
use wasm_bindgen::prelude::*;

/**
 * 从视频提取帧 - WebAssembly导出函数
 *
 * 这个函数是提供给JavaScript前端调用的主要接口。
 * 它接收原始视频数据的指针和长度，以及提取帧的时间点，
 * 然后返回一个包含处理结果或错误信息的VideoResult对象。
 *
 * @param input_ptr - 输入视频数据的指针
 * @param input_len - 输入视频数据的长度
 * @param time_sec - 提取帧的时间点(秒)
 * @returns 包含结果或错误信息的VideoResult对象
 */
#[wasm_bindgen(js_name = extractVideoFrame)]
pub fn extract_video_frame(input_ptr: *const u8, input_len: usize, time_sec: f64) -> VideoResult {
    // 从指针创建安全的切片引用
    // 注意：这是不安全操作，但在WASM环境中是必要的，因为我们需要访问JS传递的内存
    let input_data = unsafe { slice::from_raw_parts(input_ptr, input_len) };

    // 调用视频处理器提取帧 - 使用内存数据版的函数
    match video_processor::extract_frame_from_memory(input_data, time_sec) {
        Ok(buffer) => {
            // 处理成功，返回结果
            VideoResult::success(buffer)
        }
        Err(e) => {
            // 处理失败，记录错误并返回错误结果
            log_error(&e);
            VideoResult::error(e.code, &e.message)
        }
    }
}

// 额外可能需要的辅助函数

/**
 * 获取库的版本信息
 *
 * 提供给前端查询当前WASM库的版本号
 *
 * @returns 版本号字符串
 */
#[wasm_bindgen(js_name = getVersion)]
pub fn get_version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

/**
 * 检查FFmpeg和WASM环境是否正常
 *
 * 这个函数用于前端检查WASM环境和FFmpeg是否正确初始化
 *
 * @returns 如果一切正常则返回true
 */
#[wasm_bindgen(js_name = checkEnvironment)]
pub fn check_environment() -> bool {
    // 初始化FFmpeg测试环境是否正常
    crate::ffmpeg_init::initialize();
    true
}
