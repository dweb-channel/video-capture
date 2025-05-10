// 对于集成测试，我们使用主库名称引用我们的库
extern crate video_capture_wasm;

mod tests {
    // 使用库名称导入模块
    use video_capture_wasm::error::VideoErrorCode;
    use video_capture_wasm::video_processor;
    use video_capture_wasm::ffmpeg_init;
    use std::fs::File;
    use std::io::Write;
    use std::path::Path;
    // 导入image库，用于保存PNG/JPEG格式图片
    use image::{ImageBuffer, Rgb};

    // 测试 FFmpeg 初始化功能
    #[test]
    fn test_ffmpeg_initialization() {
        // 多次调用初始化函数，确保不会出现问题
        ffmpeg_init::initialize();
        ffmpeg_init::initialize();
        // 如果没有异常抛出，则测试通过
    }

    // 测试提取视频帧并保存为图片
    #[test]
    #[ignore = "需要测试视频文件"]
    fn test_extract_frame_real() {
        // 确保测试资源目录存在
        ensure_test_resources_dir();
        // 获取测试视频文件路径
        let video_path = get_test_resources_path("sample.mp4");
        println!("video_path={video_path}");

        // 从视频中提取第1秒的帧
        let result = video_processor::extract_frame(&video_path, 19.0);

        // 验证是否成功提取了帧
        assert!(result.is_ok(), "帧提取失败: {:?}", result.err());

        // 提取帧数据并验证
        let frame_data = result.unwrap();
        assert!(!frame_data.is_empty(), "提取的帧数据不应为空");

        // 从数据中推测图片尺寸
        let (width, height) = guess_image_dimensions(&frame_data);
        println!("Guessed image dimensions: {}x{}", width, height);

        // 验证尺寸是否合理
        assert!(width > 0 && height > 0, "图片尺寸推测失败");
        assert_eq!(
            width * height * 3,
            frame_data.len() as u32,
            "数据长度与推测尺寸不匹配"
        );

        // 同时保存PPM和PNG格式图片
        // 1. 保存PPM格式
        let ppm_path = get_test_resources_path("extracted_frame.ppm");
        if let Err(e) = save_rgb_as_ppm(&frame_data, width, height, &ppm_path) {
            println!("警告: 无法保存PPM格式图片: {}", e);
        }

        // 2. 保存PNG格式
        {
            let png_path = get_test_resources_path("extracted_frame.png");
            match save_rgb_as_png(&frame_data, width, height, &png_path) {
                Ok(_) => println!("成功保存PNG格式图片到: {}", png_path),
                Err(e) => println!("警告: 无法保存PNG格式图片: {}", e),
            }
        }

        println!("\n\n==============================================================");
        println!("成功提取视频帧!");
        println!("图片尺寸: {}x{}", width, height);
        println!("图片数据长度: {} 字节", frame_data.len());
        println!("图片已保存到: {}", get_test_resources_path(""));
        println!("==============================================================\n");
    }

    // 测试视频帧提取功能 - 模拟测试（不实际执行FFmpeg）
    #[test]
    fn test_extract_frame_mock() {
        // 这是一个简单的模拟测试，验证函数入口参数处理
        // 使用不存在的文件路径测试错误处理
        let non_existent_path = "./non_existent_video.mp4";
        let result = video_processor::extract_frame(non_existent_path, 0.0);

        // 应该返回错误，因为文件不存在
        assert!(result.is_err(), "不存在的文件应该返回错误");

        // 检查错误类型是否正确 - 可能是InvalidInput或其他类型的错误
        if let Err(e) = result {
            // 错误应该在预期范围内
            assert!(
                matches!(
                    e.code,
                    VideoErrorCode::InvalidInput | VideoErrorCode::FFmpegError
                ),
                "期望文件打开相关错误但收到: {:?}",
                e.code
            );
        }
    }

    // 测试边界条件和错误处理
    #[test]
    #[ignore = "需要测试视频文件"]
    fn test_extract_frame_error_cases() {
        // 确保测试资源目录存在
        ensure_test_resources_dir();
        // 测试超出视频长度的时间点
        let video_path: String = get_test_resources_path("sample.mp4");

        let result = video_processor::extract_frame(&video_path, 9999.0);

        // 应该返回 FrameNotFound 错误
        assert!(result.is_err());
        if let Err(e) = result {
            assert_eq!(e.code, VideoErrorCode::FrameNotFound);
        }

        // 测试负时间点（如果实现允许，可能会返回第一帧；否则应返回错误）
        let result = video_processor::extract_frame(&video_path, -1.0);
        // 验证结果（根据具体实现可能会有不同的预期结果）
        // 这里假设负时间点会被视为无效输入
        assert!(result.is_err());
    }

      // 辅助函数：获取测试资源目录路径
      fn get_test_resources_path(filename: &str) -> String {
        // 集成测试中资源的路径应从项目根目录开始
        if filename.is_empty() {
            "./tests/test_resources".to_string()
        } else {
            format!("./tests/test_resources/{}", filename)
        }
    }
    
    // 辅助函数：创建测试资源目录（如果不存在）
    fn ensure_test_resources_dir() {
        let path = get_test_resources_path("");
        if !Path::new(&path).exists() {
            std::fs::create_dir_all(&path).expect("无法创建测试资源目录");
        }
    }

    // 辅助函数：将RGB数据保存为PNG格式图片和PPM格式图片
    // 使用image库保存PNG图片
    fn save_rgb_as_png(
        rgb_data: &[u8],
        width: u32,
        height: u32,
        filename: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        // 验证数据大小是否正确
        if rgb_data.len() != (width * height * 3) as usize {
            return Err(Box::new(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!(
                    "数据大小不匹配: 期望 {} 字节，实际 {} 字节",
                    width * height * 3,
                    rgb_data.len()
                ),
            )));
        }

        // 创建一个新的RGB图像缓冲区
        let mut img = ImageBuffer::<Rgb<u8>, Vec<u8>>::new(width, height);

        // 将原始的RGB数据复制到图像缓冲区
        // 注意：原始数据是按行存储的，image库使用像素索引
        for y in 0..height {
            for x in 0..width {
                let idx = ((y * width + x) * 3) as usize;
                if idx + 2 < rgb_data.len() {
                    let r = rgb_data[idx];
                    let g = rgb_data[idx + 1];
                    let b = rgb_data[idx + 2];
                    img.put_pixel(x, y, Rgb([r, g, b]));
                }
            }
        }

        // 保存图像
        img.save(filename)?;
        Ok(())
    }

    // 简单的PPM保存函数
    fn save_rgb_as_ppm(
        rgb_data: &[u8],
        width: u32,
        height: u32,
        filename: &str,
    ) -> std::io::Result<()> {
        // 创建文件
        let mut file = File::create(filename)?;

        // 写入PPM头部
        // P6代表二进制RGB格式，255表示每个颜色分量的最大值
        writeln!(file, "P6")?;
        writeln!(file, "{} {}", width, height)?;
        writeln!(file, "255")?;

        // 写入RGB数据
        file.write_all(rgb_data)?;

        Ok(())
    }

    // 辅助函数：从原始数据推测图片尺寸
    // 这个函数基于数据大小和RGB格式（每像素3字节）推测图片尺寸
    fn guess_image_dimensions(data: &[u8]) -> (u32, u32) {
        // 假设图片是16:9宽高比的常见分辨率
        // 根据数据大小和每像素3字节计算像素数
        let total_pixels = data.len() / 3;
        let width_options = [1280, 1920, 640, 320]; // 常见宽度

        for &width in &width_options {
            let height = total_pixels as u32 / width;
            // 检查这个尺寸是否合理
            if width * height * 3 == data.len() as u32 {
                return (width, height);
            }
        }

        // 如果上面的常见尺寸都不匹配，尝试计算一个接近正方形的尺寸
        let sqrt_pixels = (total_pixels as f64).sqrt() as u32;
        (sqrt_pixels, total_pixels as u32 / sqrt_pixels)
    }
}
