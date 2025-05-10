use std::error::Error;
use std::fmt;
use wasm_bindgen::prelude::*;

// 仅在 Rust 内部使用
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum VideoErrorCode {
    Unknown = 0,       // 未知错误
    InitFailed = 1,    // 初始化失败
    NoVideoStream = 2, // 未找到视频流
    DecoderFailed = 3, // 解码器问题
    FrameNotFound = 4, // 帧未找到
    InvalidInput = 5,  // 无效输入
    SeekFailed = 6,    // 定位失败
    FFmpegError = 7,   // FFmpeg错误
}

// VideoErrorCode 的常规方法实现
impl VideoErrorCode {
    // 转换为字符串
    pub fn to_string(&self) -> String {
        match self {
            VideoErrorCode::Unknown => "未知错误".to_string(),
            VideoErrorCode::InitFailed => "FFmpeg初始化失败".to_string(),
            VideoErrorCode::NoVideoStream => "未找到视频流".to_string(),
            VideoErrorCode::DecoderFailed => "解码器异常".to_string(),
            VideoErrorCode::FrameNotFound => "帧未找到".to_string(),
            VideoErrorCode::InvalidInput => "无效的输入数据".to_string(),
            VideoErrorCode::SeekFailed => "定位帧失败".to_string(),
            VideoErrorCode::FFmpegError => "FFmpeg内部错误".to_string(),
        }
    }

    // 获取错误代码
    pub fn get_code(&self) -> u32 {
        *self as u32
    }
}

// 内部使用的简化错误类型
#[derive(Debug)]
pub struct VideoError {
    pub code: VideoErrorCode,
    pub message: String,
}

impl VideoError {
    // 创建新的错误
    pub fn new(code: VideoErrorCode, custom_message: Option<String>) -> Self {
        let message = match custom_message {
            Some(msg) => msg,
            None => format!("{:?}", code),
        };

        Self { code, message }
    }

    // 获取错误代码
    pub fn code(&self) -> VideoErrorCode {
        self.code
    }
}

impl fmt::Display for VideoError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl Error for VideoError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        None
    }
}

// 从 ffmpeg_next::Error 转换到 VideoError
impl From<ffmpeg_next::Error> for VideoError {
    fn from(error: ffmpeg_next::Error) -> Self {
        match error {
            ffmpeg_next::Error::StreamNotFound => Self::new(VideoErrorCode::NoVideoStream, None),
            ffmpeg_next::Error::DecoderNotFound => Self::new(VideoErrorCode::DecoderFailed, None),
            ffmpeg_next::Error::InvalidData => Self::new(VideoErrorCode::InvalidInput, None),
            _ => Self::new(
                VideoErrorCode::FFmpegError,
                Some(format!("FFmpeg错误: {}", error)),
            ),
        }
    }
}

#[wasm_bindgen]
pub struct VideoResult {
    #[wasm_bindgen(skip)]
    buffer: Vec<u8>,
    success: bool,
    error_code: u32,
    error_message: String,
}

impl VideoResult {
    // 创建成功结果
    pub fn success(data: Vec<u8>) -> Self {
        Self {
            buffer: data,
            success: true,
            error_code: VideoErrorCode::Unknown as u32,
            error_message: "".to_string(),
        }
    }

    // 创建错误结果
    pub fn error(code: VideoErrorCode, message: &str) -> Self {
        Self {
            buffer: Vec::new(),
            success: false,
            error_code: code as u32,
            error_message: message.to_string(),
        }
    }

    // 获取数据缓冲区
    pub fn get_buffer(&self) -> Vec<u8> {
        self.buffer.clone()
    }

    // 检查是否成功
    pub fn is_success(&self) -> bool {
        self.success
    }

    // 获取错误代码值
    pub fn get_error_code(&self) -> u32 {
        self.error_code
    }

    // 获取错误描述
    pub fn get_error_description(&self) -> String {
        match self.error_code {
            0 => "未知错误".to_string(),         // Unknown
            1 => "FFmpeg初始化失败".to_string(), // InitFailed
            2 => "未找到视频流".to_string(),     // NoVideoStream
            3 => "解码器异常".to_string(),       // DecoderFailed
            4 => "帧未找到".to_string(),         // FrameNotFound
            5 => "无效的输入数据".to_string(),   // InvalidInput
            6 => "定位帧失败".to_string(),       // SeekFailed
            7 => "FFmpeg内部错误".to_string(),   // FFmpegError
            _ => format!("未知错误代码: {}", self.error_code),
        }
    }

    // 获取错误消息
    pub fn get_error_message(&self) -> String {
        self.error_message.clone()
    }
}

// 辅助函数：日志记录
#[allow(dead_code)]
pub fn log_error(error: &VideoError) {
    #[cfg(target_arch = "wasm32")]
    {
        use wasm_bindgen::JsValue;
        web_sys::console::error_1(&JsValue::from_str(&format!(
            "VideoError: [{:?}] {}",
            error.code, error.message
        )));
    }

    #[cfg(not(target_arch = "wasm32"))]
    {
        eprintln!("VideoError: [{:?}] {}", error.code, error.message);
    }
}
