// video_processor.rs
// 处理视频帧提取的核心功能

use std::path::Path;

use crate::error::{VideoError, VideoErrorCode};
use crate::ffmpeg_init;

// 使用更简洁的导入方式
// 能避免代码中根据路径找不到模块的问题
use ffmpeg::{
    format::input,
    media::Type,
    software::scaling::{context::Context, flag::Flags},
    util::frame::video::Video,
};
use ffmpeg_next as ffmpeg;

/// 从视频文件中提取特定时间点的帧
///
/// # 参数
/// * `input_path` - 输入视频文件的路径
/// * `time_sec` - 要提取的帧所在的时间点（秒）
///
/// # 返回
/// * `Result<Vec<u8>, VideoError>` - 成功时返回RGB格式的帧数据，失败时返回错误
pub fn extract_frame<P: AsRef<Path>>(input_path: P, time_sec: f64) -> Result<Vec<u8>, VideoError> {
    // 确保FFmpeg已初始化
    ffmpeg_init::initialize();

    // 打开输入视频文件
    let mut ictx = match input(&input_path) {
        Ok(ctx) => ctx,
        Err(e) => {
            return Err(VideoError::new(
                VideoErrorCode::InvalidInput,
                Some(format!("无法打开视频文件: {}", e)),
            ))
        }
    };

    // 查找最佳视频流
    let video_stream = ictx
        .streams()
        .best(Type::Video)
        .ok_or(VideoError::new(VideoErrorCode::NoVideoStream, None))?;

    let video_stream_index = video_stream.index();

    // 获取解码器
    // 使用parameters方法获取流参数，然后创建解码器上下文
    let context_decoder = match ffmpeg::codec::context::Context::from_parameters(video_stream.parameters()) {
        Ok(context) => context,
        Err(e) => {
            return Err(VideoError::new(
                VideoErrorCode::DecoderFailed,
                Some(format!("无法创建解码器上下文: {}", e)),
            ))
        }
    };
    
    // 从上下文创建视频解码器
    let mut decoder = match context_decoder.decoder().video() {
        Ok(dec) => dec,
        Err(e) => {
            return Err(VideoError::new(
                VideoErrorCode::DecoderFailed,
                Some(format!("无法创建解码器: {}", e)),
            ))
        }
    };

    // 计算目标时间戳
    let time_base = video_stream.time_base();
    let target_ts =
        (time_sec * f64::from(time_base.denominator()) / f64::from(time_base.numerator())) as i64;

    // 定位到目标时间戳
    // 注意: 我们使用的是进行时间定位的优化方法
    if let Err(e) = ictx.seek(
        target_ts,
        std::ops::Range {
            start: 0,
            end: target_ts,
        },
    ) {
        return Err(VideoError::new(
            VideoErrorCode::SeekFailed,
            Some(format!("无法定位到目标时间点: {}", e)),
        ));
    }

    // 创建缩放器，将帧转换为 RGB24 格式
    let mut scaler = match Context::get(
        decoder.format(),
        decoder.width(),
        decoder.height(),
        ffmpeg::util::format::Pixel::RGB24,
        decoder.width(),
        decoder.height(),
        Flags::BILINEAR,
    ) {
        Ok(s) => s,
        Err(e) => {
            return Err(VideoError::new(
                VideoErrorCode::FFmpegError,
                Some(format!("创建缩放器失败: {}", e)),
            ))
        }
    };

    // 函数：处理已解码的帧
    let mut process_frame = |frame: &Video| -> Result<Vec<u8>, VideoError> {
        // 将帧数据转换为 RGB 格式
        let mut rgb_frame = Video::empty();
        if let Err(e) = scaler.run(frame, &mut rgb_frame) {
            return Err(VideoError::new(
                VideoErrorCode::FFmpegError,
                Some(format!("颜色转换失败: {}", e)),
            ));
        }

        // 提取RGB数据
        let data = rgb_frame.data(0);
        let stride = rgb_frame.stride(0);
        let height = rgb_frame.height();

        // 缓存通常包含项对齐字节，因此我们需要通过展平行数据来清除它们
        let mut result = Vec::with_capacity(stride * height as usize);
        for i in 0..height {
            let line_start = i as usize * stride;
            let line_end = line_start + rgb_frame.width() as usize * 3; // RGB每像素三字节
            result.extend_from_slice(&data[line_start..line_end]);
        }

        Ok(result)
    };

    // 读取并处理帧
    let mut decoded_frame = Video::empty();

    // 处理包起来，直到我们发现一个帧或数据包结束
    for (stream, packet) in ictx.packets() {
        if stream.index() == video_stream_index {
            // 将包发送给解码器
            if let Err(e) = decoder.send_packet(&packet) {
                return Err(VideoError::new(
                    VideoErrorCode::DecoderFailed,
                    Some(format!("发送数据包失败: {}", e)),
                ));
            }

            // 从解码器中接收帧
            while decoder.receive_frame(&mut decoded_frame).is_ok() {
                // 获取帧的时间戳
                let timestamp = decoded_frame.timestamp();

                // 如果帧的时间戳等于或大于目标时间戳，或者没有时间戳，则处理该帧
                if timestamp.is_none() || timestamp.unwrap() >= target_ts {
                    // 找到目标帧，直接返回处理结果
                    return process_frame(&decoded_frame);
                }
            }
        }
    }

    // 告诉解码器没有更多数据包，并尝试从中获取最后的帧
    if let Err(e) = decoder.send_eof() {
        return Err(VideoError::new(
            VideoErrorCode::DecoderFailed,
            Some(format!("发送EOF失败: {}", e)),
        ));
    }

    // 接收解码器中的任何剩余帧
    while decoder.receive_frame(&mut decoded_frame).is_ok() {
        let timestamp = decoded_frame.timestamp();
        if timestamp.is_none() || timestamp.unwrap() >= target_ts {
            // 找到合适的帧，返回处理结果
            return process_frame(&decoded_frame);
        }
    }

    // 没有找到合适的帧，返回特定错误
    Err(VideoError::new(VideoErrorCode::FrameNotFound, None))
}

// 从内存中的视频数据提取帧
// 这个函数将数据写入临时文件，然后使用文件路径版的extract_frame函数
// 这是为了保持与原有二进制数据接口的兼容性
pub fn extract_frame_from_memory(input_data: &[u8], time_sec: f64) -> Result<Vec<u8>, VideoError> {
    // 确保FFmpeg已初始化
    ffmpeg_init::initialize();

    // 创建一个临时文件来存储数据
    let temp_dir = std::env::temp_dir();
    let temp_file_path = temp_dir.join(format!(
        "video_capture_temp_{}.mp4",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis()
    ));

    // 将数据写入临时文件
    match std::fs::write(&temp_file_path, input_data) {
        Ok(_) => {
            // 文件写入成功，调用文件路径版的函数
            let result = extract_frame(&temp_file_path, time_sec);

            // 删除临时文件
            let _ = std::fs::remove_file(&temp_file_path); // 忽略清理错误

            result
        }
        Err(e) => Err(VideoError::new(
            VideoErrorCode::InvalidInput,
            Some(format!("无法写入临时文件: {}", e)),
        )),
    }
}
