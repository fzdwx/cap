# Cap - 屏幕录制工具

这是一个使用 Rust 编写的跨平台屏幕录制工具，使用 `xcap` 库进行屏幕捕获，使用 `ffmpeg` 进行视频编码。

## 功能特性

- 🎥 高质量屏幕录制
- 🚀 使用 H.264 编码器进行高效压缩
- 🔧 可配置的录制参数（分辨率、帧率等）
- 💻 跨平台支持（Linux、macOS、Windows）

## 依赖要求

### 系统依赖

在 Linux 上，需要安装以下依赖：

```bash
# Ubuntu/Debian
sudo apt-get install pkg-config libclang-dev libxcb1-dev libxrandr-dev libdbus-1-dev libpipewire-0.3-dev libwayland-dev libegl-dev

# Alpine
sudo apk add pkgconf llvm19-dev clang19-dev libxcb-dev libxrandr-dev dbus-dev pipewire-dev wayland-dev mesa-dev

# ArchLinux
sudo pacman -S base-devel clang libxcb libxrandr dbus libpipewire
```

### FFmpeg

确保系统已安装 FFmpeg 开发库：

```bash
# Ubuntu/Debian
sudo apt-get install libavcodec-dev libavformat-dev libavutil-dev libswscale-dev

# macOS (使用 Homebrew)
brew install ffmpeg

# Windows
# 下载 FFmpeg 开发库并设置环境变量
```

## 使用方法

### 基本录制

```rust
use std::{thread, time::Duration, sync::mpsc};
use xcap::Monitor;

mod video_encoder;
use video_encoder::VideoEncoder;

pub struct Frame {
    pub width: u32,
    pub height: u32,
    pub raw: Vec<u8>,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 获取显示器
    let monitor = Monitor::from_point(100, 100)?;
    let (video_recorder, sx) = monitor.video_recorder()?;

    // 创建视频编码器
    let (frame_tx, frame_rx) = mpsc::channel::<Frame>();

    // 启动编码线程
    let encoder_handle = thread::spawn(move || {
        let mut encoder = VideoEncoder::new("output.mp4", 1920, 1080, 30).unwrap();
        while let Ok(frame) = frame_rx.recv() {
            encoder.encode_frame(&frame).unwrap();
        }
        encoder.finish().unwrap();
    });

    // 启动帧接收线程
    thread::spawn(move || {
        while let Ok(frame) = sx.recv() {
            let our_frame = Frame {
                width: frame.width,
                height: frame.height,
                raw: frame.raw,
            };
            frame_tx.send(our_frame).unwrap();
        }
    });

    // 开始录制
    video_recorder.start()?;
    thread::sleep(Duration::from_secs(5)); // 录制5秒
    video_recorder.stop()?;

    encoder_handle.join().unwrap();
    Ok(())
}
```

### 运行示例

```bash
# 编译并运行
cargo run

# 运行简单示例
cargo run --example simple_recording
```

## 项目结构

```
src/
├── main.rs              # 主程序入口
├── video_encoder.rs     # FFmpeg 视频编码器
examples/
├── simple_recording.rs  # 简单录制示例
```

## 配置选项

### VideoEncoder 参数

- `output_path`: 输出视频文件路径
- `width`: 视频宽度
- `height`: 视频高度
- `fps`: 帧率

### 编码设置

默认使用以下 H.264 编码设置：
- 预设: `fast`
- CRF: `23` (恒定质量模式)
- 像素格式: `YUV420P`

## 输出格式

生成的视频文件具有以下特性：
- 格式: MP4
- 编解码器: H.264 (AVC)
- 像素格式: YUV420P
- 兼容大多数播放器和平台