mod test;

use std::ffi::CStr;
use std::os::raw::{c_char, c_int};
use std::slice;
use wasm_bindgen::prelude::*;

// 只引入需要的FFmpeg模块
use ffmpeg_next::codec; // libavcodec
use ffmpeg_next::format; // libavformat
use ffmpeg_next::software::scaling; // libswscale

// 自定义错误类型
#[derive(Debug)]
pub enum FrameExtractError {
    FFmpegError(ffmpeg_next::Error),
    FrameNotFound,
}

impl From<ffmpeg_next::Error> for FrameExtractError {
    fn from(err: ffmpeg_next::Error) -> Self {
        FrameExtractError::FFmpegError(err)
    }
}

impl std::fmt::Display for FrameExtractError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FrameExtractError::FFmpegError(err) => write!(f, "FFmpeg error: {}", err),
            FrameExtractError::FrameNotFound => write!(f, "Frame not found at specified time"),
        }
    }
}

impl std::error::Error for FrameExtractError {}

// 配置选项
#[derive(Debug, Clone, Copy)]
pub struct FrameExtractOptions {
    pub pixel_format: PixelFormat,
    pub scaling_method: ScalingMethod,
    pub width: Option<u32>,  // 目标宽度，None表示保持原始宽度
    pub height: Option<u32>, // 目标高度，None表示保持原始高度
}

#[derive(Debug, Clone, Copy)]
pub enum PixelFormat {
    RGB24,
    RGBA,
    BGR24,
    GRAY8,
}

#[derive(Debug, Clone, Copy)]
pub enum ScalingMethod {
    Bilinear,
    Bicubic,
    AreaBased,
    Fast,
}

impl Default for FrameExtractOptions {
    fn default() -> Self {
        Self {
            pixel_format: PixelFormat::RGB24,
            scaling_method: ScalingMethod::Bilinear,
            width: None,
            height: None,
        }
    }
}

// 初始化FFmpeg
fn init() {
    static INIT: std::sync::Once = std::sync::Once::new();
    INIT.call_once(|| {
        ffmpeg_next::init().unwrap();
        // 设置日志级别为安静，减少不必要的输出
        ffmpeg_next::util::log::set_level(ffmpeg_next::util::log::Level::Quiet);
    });
}

// 将PixelFormat转换为ffmpeg的Pixel格式
fn to_ffmpeg_pixel_format(format: PixelFormat) -> ffmpeg_next::util::format::Pixel {
    match format {
        PixelFormat::RGB24 => ffmpeg_next::util::format::Pixel::RGB24,
        PixelFormat::RGBA => ffmpeg_next::util::format::Pixel::RGBA,
        PixelFormat::BGR24 => ffmpeg_next::util::format::Pixel::BGR24,
        PixelFormat::GRAY8 => ffmpeg_next::util::format::Pixel::GRAY8,
    }
}

// 将ScalingMethod转换为ffmpeg的scaling flag
fn to_ffmpeg_scaling_flag(method: ScalingMethod) -> scaling::flag::Flags {
    match method {
        ScalingMethod::Bilinear => scaling::flag::Flags::BILINEAR,
        ScalingMethod::Bicubic => scaling::flag::Flags::BICUBIC,
        ScalingMethod::AreaBased => scaling::flag::Flags::AREA,
        ScalingMethod::Fast => scaling::flag::Flags::FAST_BILINEAR,
    }
}

/// 从视频数据中提取指定时间点的帧
/// 
/// # 参数
/// * `input_data` - 视频数据的字节切片
/// * `time_sec` - 要提取帧的时间点（秒）
/// * `options` - 可选的提取配置，如像素格式、缩放方法等
/// 
/// # 返回
/// * `Result<Vec<u8>, FrameExtractError>` - 成功返回帧数据，失败返回错误
fn extract_frame(
    input_data: &[u8], 
    time_sec: f64,
    options: FrameExtractOptions,
) -> Result<Vec<u8>, FrameExtractError> {
    // 确保FFmpeg已初始化
    init();

    // 创建内存输入上下文
    let io_context = ffmpeg_next::format::io::Input::from_buffer(input_data)?;
    let mut input_ctx = format::context::Input::from_io(io_context)?;

    // 查找最佳视频流
    let stream_index = input_ctx
        .streams()
        .best(ffmpeg_next::media::Type::Video)
        .ok_or(FrameExtractError::FrameNotFound)?
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

    // 计算输出分辨率
    let calculate_dimensions = |orig_width: u32, orig_height: u32| -> (u32, u32) {
        let out_width = options.width.unwrap_or(orig_width);
        let out_height = options.height.unwrap_or(orig_height);
        (out_width, out_height)
    };

    // 处理帧需要一个闭包，以避免代码重复
    let process_frame = |frame: &ffmpeg_next::frame::Video| -> Result<Vec<u8>, FrameExtractError> {
        // 获取原始尺寸
        let width = frame.width();
        let height = frame.height();
        
        // 计算输出尺寸
        let (out_width, out_height) = calculate_dimensions(width, height);
        
        // 创建或获取缩放器
        let scaler = scaler.get_or_insert_with(|| {
            scaling::context::Context::get(
                frame.format(),
                width,
                height,
                to_ffmpeg_pixel_format(options.pixel_format),
                out_width,
                out_height,
                to_ffmpeg_scaling_flag(options.scaling_method),
            )
            .unwrap()
        });

        // 创建输出帧
        let mut output_frame = ffmpeg_next::frame::Video::empty();
        scaler.run(frame, &mut output_frame)?;

        // 计算每行字节数和总大小
        let stride = output_frame.stride(0);
        let frame_height = output_frame.height() as usize;
        let bytes_per_pixel = match options.pixel_format {
            PixelFormat::RGB24 | PixelFormat::BGR24 => 3,
            PixelFormat::RGBA => 4,
            PixelFormat::GRAY8 => 1,
        };
        
        // 计算实际每行的有效数据大小（不包括可能的padding）
        let line_size = out_width as usize * bytes_per_pixel;
        
        // 预分配足够的空间
        let mut result = Vec::with_capacity(line_size * frame_height);
        let data = output_frame.data(0);
        
        // 只复制每行实际需要的字节，跳过padding
        for i in 0..frame_height {
            let line_start = i * stride;
            // 只取每行实际使用的字节（可能小于stride，因为可能有padding）
            let line_end = line_start + line_size;
            result.extend_from_slice(&data[line_start..line_end]);
        }

        Ok(result)
    };

    // 循环读取帧直到找到目标时间点的帧
    while input_ctx.read(&mut packet).is_ok() {
        if packet.stream() != stream_index {
            continue;
        }

        decoder.send_packet(&packet)?;
        
        // 接收帧
        while decoder.receive_frame(&mut decoded_frame).is_ok() {
            let timestamp = decoded_frame.timestamp();
            if timestamp.is_none() || timestamp.unwrap() >= target_ts {
                return process_frame(&decoded_frame);
            }
        }
    }

    // 冲洗解码器以获取任何缓冲的帧
    decoder.send_eof()?;
    while decoder.receive_frame(&mut decoded_frame).is_ok() {
        let timestamp = decoded_frame.timestamp();
        if timestamp.is_none() || timestamp.unwrap() >= target_ts {
            return process_frame(&decoded_frame);
        }
    }

    Err(FrameExtractError::FrameNotFound)
}

/// 封装后的API，用于WebAssembly导出
/// 
/// # 参数
/// * `input_ptr` - 指向视频数据的指针
/// * `input_len` - 视频数据的长度
/// * `time_sec` - 要提取帧的时间点（秒）
/// 
/// # 返回
/// * `Vec<u8>` - 成功时返回帧数据，失败时返回空向量
#[wasm_bindgen]
pub fn get_video_frame(input_ptr: *const u8, input_len: usize, time_sec: f64) -> Vec<u8> {
    unsafe {
        // 将原始指针转换为切片
        let input_data = slice::from_raw_parts(input_ptr, input_len);
        // 使用默认配置
        match extract_frame(input_data, time_sec, FrameExtractOptions::default()) {
            Ok(buffer) => buffer,
            Err(_) => {
                Vec::new() // 返回空表示错误
            }
        }
    }
}

/// 额外的API，允许指定输出格式，为未来扩展做准备
/// 
/// # 参数
/// * `input_ptr` - 指向视频数据的指针
/// * `input_len` - 视频数据的长度
/// * `time_sec` - 要提取帧的时间点（秒）
/// * `format` - 输出像素格式：0=RGB24, 1=RGBA, 2=BGR24, 3=GRAY8
/// * `width` - 输出宽度，0表示保持原始宽度
/// * `height` - 输出高度，0表示保持原始高度
/// 
/// # 返回
/// * `Vec<u8>` - 成功时返回帧数据，失败时返回空向量
#[wasm_bindgen]
pub fn get_video_frame_with_options(
    input_ptr: *const u8, 
    input_len: usize, 
    time_sec: f64,
    format: u8,
    scaling: u8,
    width: u32,
    height: u32,
) -> Vec<u8> {
    unsafe {
        // 将原始指针转换为切片
        let input_data = slice::from_raw_parts(input_ptr, input_len);
        
        // 解析像素格式
        let pixel_format = match format {
            0 => PixelFormat::RGB24,
            1 => PixelFormat::RGBA,
            2 => PixelFormat::BGR24,
            3 => PixelFormat::GRAY8,
            _ => PixelFormat::RGB24, // 默认使用RGB24
        };
        
        // 解析缩放方法
        let scaling_method = match scaling {
            0 => ScalingMethod::Bilinear,
            1 => ScalingMethod::Bicubic,
            2 => ScalingMethod::AreaBased,
            3 => ScalingMethod::Fast,
            _ => ScalingMethod::Bilinear, // 默认使用双线性
        };
        
        // 创建配置
        let options = FrameExtractOptions {
            pixel_format,
            scaling_method,
            width: if width == 0 { None } else { Some(width) },
            height: if height == 0 { None } else { Some(height) },
        };
        
        // 提取帧
        match extract_frame(input_data, time_sec, options) {
            Ok(buffer) => buffer,
            Err(_) => {
                Vec::new() // 返回空表示错误
            }
        }
    }
}
