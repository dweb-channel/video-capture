# 视频抽帧库 (Video Capture Library)

这个库提供了一个简单的视频抽帧功能，可以从视频文件或流中提取特定时间点的帧。


## 前端如何使用

```ts
// 从视频获取指定时间点的帧
const result = videoModule.extractVideoFrame(videoDataPtr, videoLength, timeInSeconds);

if (result.isSuccess()) {
  // 成功获取帧数据
  const frameBuffer = result.getBuffer();
  // 处理帧数据...例如创建图像
  const blob = new Blob([frameBuffer], { type: 'image/rgb' });
  // ...
} else {
  // 处理错误情况
  const errorCode = result.getErrorCode();
  const errorMessage = result.getErrorMessage();
  
  console.error(`视频处理失败: ${errorMessage} (代码: ${errorCode.toString()})`);
}
```

## 特性

- 只使用FFmpeg的必要库：libavformat、libavcodec、libswscale和libavutil
- 支持WebAssembly (WASM) 导出
- 提供内存中视频处理，无需写入临时文件
- 支持多种视频格式

## 实现细节

该库使用了`ffmpeg-next` Rust绑定，并且只引入了必要的FFmpeg组件：

- **libavformat** - 用于读取视频文件或流（通过`format`特性）
- **libavcodec** - 用于解码视频帧（通过`codec`特性）
- **libswscale** - 用于像素格式转换（通过`software-scaling`特性）
- **libavutil** - 提供通用工具函数（在 ffmpeg-next 7.1.0 中已包含在其他模块中）

在`Cargo.toml`中，我们通过特性标志限制了只使用这些必要的组件，并添加了`build`特性以便在编译时自动构建FFmpeg：

```toml
ffmpeg-next = { version = "7.1", default-features = false, features = ["format", "codec", "software-scaling", "build"] }
```

使用`build`特性意味着在编译时会从源代码构建FFmpeg，而不是依赖系统安装的FFmpeg库。这样可以确保项目在没有预先安装FFmpeg的环境中也能正常编译。

## 使用方法

该库提供了一个简单的API用于提取视频帧：

```rust
pub fn extract_video_frame(
    input_ptr: *const u8,  // 指向视频数据的指针
    input_len: usize,      // 视频数据长度
    time_sec: f64          // 要提取的时间点（秒）
) -> VideoResult        // 返回RGB格式的帧数据
```

## 编译说明

要编译此库，您需要：

1. 安装Rust和Cargo
2. 如果不使用`build`特性，则需要安装FFmpeg开发库
3. 运行 `cargo build --release`

对于WebAssembly目标，请使用：

```bash
cargo build --target wasm32-unknown-unknown --release
```

## how to build

1. rustup target add wasm32-unknown-unknown
1. install [wasm-bindgen]() `cargo install wasm-bindgen-cli`
1. install [wasm-pack](https://rustwasm.github.io/wasm-pack/installer/)
   `cargo install wasm-pack`
1. install [tsc](http://npmjs.com/package/typescript) `npm install -g typescript`
1. install [deno](https://deno.com/)
   ```
   curl -fsSL https://deno.land/install.sh | sh # macos or linux
   irm https://deno.land/install.ps1 | iex # windows
   ```
1. run script: `deno task build`
   > output to [pkg](./pkg) folder