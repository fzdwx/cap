use std::{thread, time::Duration, sync::mpsc, time::Instant};
use xcap::Monitor;

// 由于这是一个示例文件，我们需要包含 video_encoder 模块
// 在实际项目中，这应该是一个独立的 crate 或模块
#[path = "../src/video_encoder.rs"]
mod video_encoder;
use video_encoder::VideoEncoder;

// Frame 结构体定义
pub struct Frame {
    pub width: u32,
    pub height: u32,
    pub raw: Vec<u8>,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("开始屏幕录制示例...");
    
    // 获取主显示器
    let monitor = Monitor::from_point(100, 100)?;
    println!("使用显示器: {}x{}", monitor.width()?, monitor.height()?);

    // 创建视频录制器
    let (video_recorder, sx) = monitor.video_recorder()?;

    // 创建帧传输通道
    let (frame_tx, frame_rx) = mpsc::channel::<Frame>();
    let frame_tx_clone = frame_tx.clone();

    // 启动视频编码线程
    let encoder_handle = thread::spawn(move || {
        let mut encoder = VideoEncoder::new("recording.mp4", 1920, 1080, 30).unwrap();
        let mut frame_count = 0;
        
        println!("视频编码器已启动");
        
        while let Ok(frame) = frame_rx.recv() {
            if let Err(e) = encoder.encode_frame(&frame) {
                eprintln!("编码帧时出错: {}", e);
                break;
            }
            frame_count += 1;
            if frame_count % 10 == 0 {
                println!("已编码 {} 帧", frame_count);
            }
        }
        
        println!("开始完成编码...");
        if let Err(e) = encoder.finish() {
            eprintln!("完成编码时出错: {}", e);
        }
        println!("视频编码完成，共编码 {} 帧", frame_count);
    });

    // 帧接收线程
    let frame_handle = thread::spawn(move || {
        let mut frame_count = 0;
        let start_time = Instant::now();
        let timeout = Duration::from_secs(8); // 8秒超时
        
        loop {
            match sx.recv_timeout(Duration::from_millis(100)) {
                Ok(frame) => {
                    frame_count += 1;
                    
                    // 将 xcap 的帧转换为我们的 Frame 结构
                    let our_frame = Frame {
                        width: frame.width,
                        height: frame.height,
                        raw: frame.raw,
                    };
                    
                    // 发送帧到编码器
                    if frame_tx_clone.send(our_frame).is_err() {
                        println!("编码器已关闭，停止发送帧");
                        break;
                    }
                }
                Err(mpsc::RecvTimeoutError::Timeout) => {
                    // 检查是否超时
                    if start_time.elapsed() > timeout {
                        println!("帧接收超时，停止接收");
                        break;
                    }
                    continue;
                }
                Err(mpsc::RecvTimeoutError::Disconnected) => {
                    println!("帧接收通道关闭");
                    break;
                }
            }
        }
        println!("帧接收线程结束，共接收 {} 帧", frame_count);
    });

    // 开始录制
    println!("开始录制 (3秒)...");
    video_recorder.start()?;
    thread::sleep(Duration::from_secs(3));
    println!("停止录制");
    video_recorder.stop()?;
    
    // 等待处理完成
    frame_handle.join().unwrap();
    drop(frame_tx); // 关闭发送端
    encoder_handle.join().unwrap();
    
    println!("录制完成！视频已保存为 recording.mp4");
    Ok(())
}
