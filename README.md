# 视频抽帧库 (Video Capture Library)

这个库提供了一个简单的视频抽帧功能，可以从视频文件或流中提取特定时间点的帧。

### 🚫注意项目废弃，	wasm32-unknown-unknown 是纯 wasm，没有任何 C ABI 支持。

建议使用 [ffmpeg.wasm](https://github.com/ffmpegwasm/ffmpeg.wasm)

```ts
import { createFFmpeg, fetchFile } from '@ffmpeg/ffmpeg';

const ffmpeg = createFFmpeg({ log: true });

await ffmpeg.load();
ffmpeg.FS('writeFile', 'input.mp4', await fetchFile(videoBlob));
await ffmpeg.run('-i', 'input.mp4', '-ss', '00:00:01', '-frames:v', '1', 'output.jpg');
const data = ffmpeg.FS('readFile', 'output.jpg');
const url = URL.createObjectURL(new Blob([data.buffer], { type: 'image/jpeg' }));
```

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


## rust使用方法

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