use ffmpeg_next as ffmpeg;
use std::path::Path;

use crate::Frame;

pub struct VideoEncoder {
    output_context: ffmpeg::format::context::Output,
    video_stream: usize,
    encoder: ffmpeg::encoder::Video,
    frame_count: i64,
}

impl VideoEncoder {
    pub fn new<P: AsRef<Path>>(
        output_path: P,
        width: u32,
        height: u32,
        fps: u32,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        // 初始化 FFmpeg
        ffmpeg::init()?;

        // 创建输出上下文
        let mut output_context = ffmpeg::format::output(&output_path)?;

        // 查找 H.264 编码器
        let codec = ffmpeg::encoder::find(ffmpeg::codec::Id::H264)
            .ok_or("找不到 H.264 编码器")?;

        // 添加视频流
        let mut video_stream = output_context.add_stream(codec)?;
        let video_stream_index = video_stream.index();

        // 配置编码器
        let mut encoder = ffmpeg::codec::context::Context::new_with_codec(codec)
            .encoder()
            .video()?;

        encoder.set_width(width);
        encoder.set_height(height);
        encoder.set_format(ffmpeg::format::Pixel::YUV420P);
        encoder.set_time_base((1, fps as i32));
        video_stream.set_time_base((1, fps as i32));

        // 设置编码器参数
        let mut dict = ffmpeg::Dictionary::new();
        dict.set("preset", "fast");
        dict.set("crf", "23");

        let encoder = encoder.open_with(dict)?;
        video_stream.set_parameters(&encoder);

        // 写入文件头
        output_context.write_header()?;

        Ok(VideoEncoder {
            output_context,
            video_stream: video_stream_index,
            encoder,
            frame_count: 0,
        })
    }

    pub fn encode_frame(&mut self, frame: &Frame) -> Result<(), Box<dyn std::error::Error>> {
        // 创建 FFmpeg 帧
        let mut ffmpeg_frame = ffmpeg::frame::Video::new(
            ffmpeg::format::Pixel::BGRA,
            frame.width,
            frame.height,
        );

        // 将原始数据复制到 FFmpeg 帧
        // xcap 通常提供 BGRA 格式的数据
        let expected_size = (frame.width * frame.height * 4) as usize;
        if frame.raw.len() != expected_size {
            return Err(format!(
                "帧数据大小不匹配: 期望 {} 字节，实际 {} 字节",
                expected_size,
                frame.raw.len()
            ).into());
        }

        ffmpeg_frame.data_mut(0)[..frame.raw.len()].copy_from_slice(&frame.raw);

        // 设置帧的时间戳
        ffmpeg_frame.set_pts(Some(self.frame_count));

        // 创建转换器将 BGRA 转换为 YUV420P
        let mut converter = ffmpeg::software::scaling::context::Context::get(
            ffmpeg::format::Pixel::BGRA,
            frame.width,
            frame.height,
            ffmpeg::format::Pixel::YUV420P,
            frame.width,
            frame.height,
            ffmpeg::software::scaling::flag::Flags::BILINEAR,
        )?;

        let mut yuv_frame = ffmpeg::frame::Video::new(
            ffmpeg::format::Pixel::YUV420P,
            frame.width,
            frame.height,
        );
        yuv_frame.set_pts(Some(self.frame_count));

        converter.run(&ffmpeg_frame, &mut yuv_frame)?;

        // 编码帧
        self.encoder.send_frame(&yuv_frame)?;
        self.receive_and_write_packets()?;

        self.frame_count += 1;
        Ok(())
    }



    fn receive_and_write_packets(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let mut packet = ffmpeg::packet::Packet::empty();
        
        while self.encoder.receive_packet(&mut packet).is_ok() {
            packet.set_stream(self.video_stream);
            packet.rescale_ts(
                self.encoder.time_base(),
                self.output_context.stream(self.video_stream).unwrap().time_base(),
            );
            packet.write_interleaved(&mut self.output_context)?;
        }
        
        Ok(())
    }

    pub fn finish(mut self) -> Result<(), Box<dyn std::error::Error>> {
        // 发送空帧以刷新编码器
        self.encoder.send_eof()?;
        self.receive_and_write_packets()?;

        // 写入文件尾
        self.output_context.write_trailer()?;

        Ok(())
    }
}
