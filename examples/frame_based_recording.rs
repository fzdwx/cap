use std::{thread, time::Duration, sync::mpsc, sync::Arc, sync::atomic::{AtomicBool, AtomicUsize, Ordering}};
use xcap::Monitor;

#[path = "../src/video_encoder.rs"]
mod video_encoder;
use video_encoder::VideoEncoder;

pub struct Frame {
    pub width: u32,
    pub height: u32,
    pub raw: Vec<u8>,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("开始基于帧数的屏幕录制示例...");
    
    // 录制参数
    let target_frames = 150; // 目标帧数（5秒 @ 30fps）
    let target_fps = 30;
    let expected_duration = target_frames as f32 / target_fps as f32;
    
    println!("目标: {} 帧，预期时长: {:.1}秒", target_frames, expected_duration);
    
    // 获取主显示器
    let monitor = Monitor::from_point(100, 100)?;
    println!("使用显示器: {}x{}", monitor.width()?, monitor.height()?);

    // 创建视频录制器
    let (video_recorder, sx) = monitor.video_recorder()?;

    // 创建帧传输通道
    let (frame_tx, frame_rx) = mpsc::channel::<Frame>();
    let frame_tx_clone = frame_tx.clone();

    // 共享状态
    let frames_received = Arc::new(AtomicUsize::new(0));
    let should_stop = Arc::new(AtomicBool::new(false));
    
    let frames_received_clone = frames_received.clone();
    let should_stop_clone = should_stop.clone();

    // 启动视频编码线程
    let encoder_handle = thread::spawn(move || {
        let mut encoder = VideoEncoder::new("frame_based_recording.mp4", 1920, 1080, target_fps).unwrap();
        let mut frame_count = 0;
        
        println!("视频编码器已启动");
        
        while let Ok(frame) = frame_rx.recv() {
            if let Err(e) = encoder.encode_frame(&frame) {
                eprintln!("编码帧时出错: {}", e);
                break;
            }
            frame_count += 1;
            if frame_count % 30 == 0 {
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
        loop {
            match sx.recv_timeout(Duration::from_millis(50)) {
                Ok(frame) => {
                    let count = frames_received_clone.fetch_add(1, Ordering::SeqCst) + 1;
                    
                    if count % 30 == 0 {
                        println!("收到第 {} 帧", count);
                    }
                    
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
                    
                    // 检查是否达到目标帧数
                    if count >= target_frames {
                        println!("达到目标帧数 {}，停止接收", target_frames);
                        should_stop_clone.store(true, Ordering::SeqCst);
                        break;
                    }
                }
                Err(mpsc::RecvTimeoutError::Timeout) => {
                    // 检查是否应该停止
                    if should_stop_clone.load(Ordering::SeqCst) {
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
        let final_count = frames_received_clone.load(Ordering::SeqCst);
        println!("帧接收线程结束，共接收 {} 帧", final_count);
    });

    // 开始录制
    println!("开始录制...");
    video_recorder.start()?;
    
    // 等待达到目标帧数或超时
    let max_wait_time = Duration::from_secs(20); // 最大等待20秒
    let start_time = std::time::Instant::now();
    
    loop {
        let current_frames = frames_received.load(Ordering::SeqCst);
        
        if current_frames >= target_frames {
            println!("达到目标帧数，停止录制");
            break;
        }
        
        if start_time.elapsed() > max_wait_time {
            println!("录制超时，停止录制");
            should_stop.store(true, Ordering::SeqCst);
            break;
        }
        
        thread::sleep(Duration::from_millis(100));
    }
    
    println!("停止录制");
    video_recorder.stop()?;
    
    // 等待处理完成
    frame_handle.join().unwrap();
    drop(frame_tx); // 关闭发送端
    encoder_handle.join().unwrap();
    
    let final_frames = frames_received.load(Ordering::SeqCst);
    let actual_duration = final_frames as f32 / target_fps as f32;
    
    println!("录制完成！");
    println!("实际帧数: {}", final_frames);
    println!("实际时长: {:.2}秒", actual_duration);
    println!("视频已保存为 frame_based_recording.mp4");
    
    Ok(())
}
