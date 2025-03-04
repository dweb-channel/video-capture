mod test;

use std::ffi::CStr;
use std::os::raw::{c_char, c_int};
use std::slice;
use wasm_bindgen::prelude::*;

// 只引入需要的FFmpeg模块
use ffmpeg_next::codec; // libavcodec
use ffmpeg_next::format; // libavformat
use ffmpeg_next::software::scaling; // libswscale

// 初始化FFmpeg
fn init() {
    static INIT: std::sync::Once = std::sync::Once::new();
    INIT.call_once(|| {
        ffmpeg_next::init().unwrap();
    });
}

#[link(wasm_import_module = "ffmpeg")]
extern "C" {
    // 自定义的极简帧提取接口
    pub fn extract_frame_to_buffer(
        input: *const c_char,
        time_sec: f64,
        out_buf: *mut u8,
        buf_size: c_int,
    ) -> c_int;
}

// 使用Rust FFmpeg绑定实现帧提取功能
fn extract_frame(input_data: &[u8], time_sec: f64) -> Result<Vec<u8>, ffmpeg_next::Error> {
    init();

    // 创建内存输入上下文
    let io_context = ffmpeg_next::format::io::Input::from_buffer(input_data)?;
    let mut input_ctx = format::context::Input::from_io(io_context)?;

    // 查找最佳视频流
    let stream_index = input_ctx
        .streams()
        .best(ffmpeg_next::media::Type::Video)
        .ok_or(ffmpeg_next::Error::StreamNotFound)?
        .index();

    // 获取解码器
    let stream = input_ctx.stream(stream_index).unwrap();
    let context_decoder = codec::context::Context::from_parameters(stream.parameters())?;
    let mut decoder = context_decoder.decoder().video()?;
    
    // 计算目标时间戳
    let time_base = stream.time_base();
    let target_ts =
        (time_sec * f64::from(time_base.denominator()) / f64::from(time_base.numerator())) as i64;

    // 寻找最近的关键帧
    input_ctx.seek(
        target_ts,
        stream_index as i32,
        ffmpeg_next::format::SeekFlag::BACKWARD,
    )?;

    // 解码帧
    let mut decoded_frame = ffmpeg_next::frame::Video::empty();
    let mut scaler = None;
    let mut packet = ffmpeg_next::packet::Packet::empty();

    while input_ctx.read(&mut packet).is_ok() {
        if packet.stream() != stream_index {
            continue;
        }

        decoder.send_packet(&packet)?;

        while decoder.receive_frame(&mut decoded_frame).is_ok() {
            let timestamp = decoded_frame.timestamp();
            if timestamp.is_none() || timestamp.unwrap() >= target_ts {
                // 转换为RGB格式
                let scaler = scaler.get_or_insert_with(|| {
                    scaling::context::Context::get(
                        decoder.format(),
                        decoder.width(),
                        decoder.height(),
                        ffmpeg_next::util::format::Pixel::RGB24,
                        decoder.width(),
                        decoder.height(),
                        scaling::flag::Flags::BILINEAR,
                    )
                    .unwrap()
                });

                let mut rgb_frame = ffmpeg_next::frame::Video::empty();
                scaler.run(&decoded_frame, &mut rgb_frame)?;

                // 将帧数据转换为Vec<u8>
                let data = rgb_frame.data(0);
                let stride = rgb_frame.stride(0);
                let height = rgb_frame.height();

                let mut result = Vec::with_capacity(stride * height as usize);
                for i in 0..height {
                    let line_start = i as usize * stride;
                    let line_end = line_start + stride;
                    result.extend_from_slice(&data[line_start..line_end]);
                }

                return Ok(result);
            }
        }
    }

    Err(ffmpeg_next::Error::Other {
        description: "Frame not found".to_string(),
    })
}

#[wasm_bindgen]
pub fn get_video_frame(input_ptr: *const u8, input_len: usize, time_sec: f64) -> Vec<u8> {
    unsafe {
        // 尝试使用Rust FFmpeg绑定
        let input_data = slice::from_raw_parts(input_ptr, input_len);
        match extract_frame(input_data, time_sec) {
            Ok(buffer) => buffer,
            Err(_) => {
                Vec::new() // 返回空表示错误
            }
        }
    }
}
