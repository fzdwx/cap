use std::{thread, time::Duration, sync::mpsc, time::Instant};
use xcap::{Frame, Monitor};

mod video_encoder;
use video_encoder::VideoEncoder;

fn main() {
    let monitor = Monitor::from_point(100, 100).unwrap();

    let (video_recorder, sx) = monitor.video_recorder().unwrap();

    // 创建一个通道用于在线程间传递帧数据
    let (frame_tx, frame_rx) = mpsc::channel::<Frame>();
    let frame_tx_clone = frame_tx.clone();

    // 启动视频编码线程
    let encoder_handle = thread::spawn(move || {
        let mut encoder = VideoEncoder::new("output.mp4", 1920, 1080, 30).unwrap();
        let mut frame_count = 0;

        while let Ok(frame) = frame_rx.recv() {
            if let Err(e) = encoder.encode_frame(&frame) {
                eprintln!("编码帧时出错: {}", e);
                break;
            }
            frame_count += 1;
            println!("已编码 {} 帧", frame_count);
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
        let timeout = Duration::from_secs(10); // 10秒超时

        loop {
            match sx.recv_timeout(Duration::from_millis(100)) {
                Ok(frame) => {
                    frame_count += 1;
                    println!("收到帧 {}: {}x{}", frame_count, frame.width, frame.height);
                    // 发送帧到编码器
                    if frame_tx_clone.send(frame).is_err() {
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

    println!("开始录制");
    video_recorder.start().unwrap();
    thread::sleep(Duration::from_secs(5)); // 录制5秒
    println!("停止录制");
    video_recorder.stop().unwrap();

    // 等待帧接收完成
    frame_handle.join().unwrap();

    // 关闭发送端，让编码器知道没有更多帧了
    drop(frame_tx);

    // 等待编码完成
    encoder_handle.join().unwrap();
}
