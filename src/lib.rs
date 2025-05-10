// 公开模块供测试使用
pub mod error;
pub mod ffmpeg_init;
pub mod video_processor;
mod wasm_interface;

// 导出公开的 API
pub use wasm_interface::extract_video_frame;
