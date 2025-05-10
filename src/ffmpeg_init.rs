// ffmpeg_init.rs
// 处理FFmpeg的初始化，确保线程安全

/// 初始化FFmpeg库
/// 
/// 此函数使用`std::sync::Once`确保FFmpeg库只被初始化一次，
/// 防止多次调用导致的潜在问题。
pub fn initialize() {
    static INIT: std::sync::Once = std::sync::Once::new();
    INIT.call_once(|| {
        // 在初始化过程中使用unwrap是合理的，因为如果FFmpeg不能初始化，
        // 整个库将无法正常工作
        ffmpeg_next::init().unwrap();
        
        // 设置FFmpeg日志级别（可选）
        #[cfg(debug_assertions)]
        ffmpeg_next::util::log::set_level(ffmpeg_next::util::log::Level::Debug);
        
        #[cfg(not(debug_assertions))]
        ffmpeg_next::util::log::set_level(ffmpeg_next::util::log::Level::Error);
    });
}
