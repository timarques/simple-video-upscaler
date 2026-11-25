use super::CHANNEL_COUNT;
use crate::frame::Frame;
use crate::reorderer::Reorderer;
use crate::video::Video;

use anyhow::{Context, Result, bail};
use crossbeam_channel::{Receiver, Sender};
use std::collections::HashMap;
use std::io::{BufWriter, Write};
use std::process::{Child, ChildStdin, Command, Stdio};

struct IoPipeline {
    process: Child,
    writer: BufWriter<ChildStdin>,
    final_frame_sender: Sender<Result<Frame>>,
    reorderer: Reorderer,
    bytes_buffer: HashMap<usize, Vec<u8>>,
}

impl IoPipeline {
    fn new(process: Child, stdin: ChildStdin, final_frame_sender: Sender<Result<Frame>>) -> Self {
        let writer = BufWriter::new(stdin);
        let reorderer = Reorderer::new();
        let bytes_buffer = HashMap::new();

        Self {
            process,
            writer,
            final_frame_sender,
            reorderer,
            bytes_buffer,
        }
    }

    fn run(mut self, data_receiver: Receiver<Result<(Frame, Vec<u8>)>>) -> Result<()> {
        for data_result in data_receiver {
            let (frame, bytes) = data_result?;
            self.buffer_frame(frame, bytes);
            self.write_ready_frames()?;
        }
        self.finalize()
    }

    fn buffer_frame(&mut self, frame: Frame, bytes: Vec<u8>) {
        self.bytes_buffer.insert(frame.index, bytes);
        self.reorderer.push(frame);
    }

    fn write_ready_frames(&mut self) -> Result<()> {
        while let Some(ready_frame) = self.reorderer.pop() {
            let frame_index = ready_frame.index;
            let ready_bytes = self
                .bytes_buffer
                .remove(&frame_index)
                .context(format!("Byte data for frame {frame_index} not in buffer"))?;

            self.writer.write_all(&ready_bytes)?;
            self.final_frame_sender.send(Ok(ready_frame))?;
        }
        Ok(())
    }

    fn finalize(self) -> Result<()> {
        drop(self.writer);

        let output = self.process.wait_with_output()?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            bail!("FFmpeg exited with a non-zero status:\n{stderr}");
        }

        if !self.bytes_buffer.is_empty() {
            let remaining_indices: Vec<_> = self.bytes_buffer.keys().collect();
            bail!("Frames left in buffer: {remaining_indices:?}");
        }
        Ok(())
    }
}

pub struct MergePhase;

impl MergePhase {
    pub fn execute(
        video: &Video,
        frame_receiver: Receiver<Result<Frame>>,
    ) -> Result<Receiver<Result<Frame>>> {
        let mut child = Self::spawn_ffmpeg_process(video)?;
        let stdin = child
            .stdin
            .take()
            .context("Failed to get FFmpeg's stdin handle")?;

        let (data_sender, data_receiver) = crossbeam_channel::bounded(CHANNEL_COUNT);
        let (final_frame_sender, final_frame_receiver) = crossbeam_channel::bounded(CHANNEL_COUNT);

        Self::spawn_serialization_workers(frame_receiver, data_sender);
        Self::spawn_io_worker(child, stdin, data_receiver, final_frame_sender);

        Ok(final_frame_receiver)
    }

    fn spawn_serialization_workers(
        frame_receiver: Receiver<Result<Frame>>,
        data_sender: Sender<Result<(Frame, Vec<u8>)>>,
    ) {
        for _ in 0..CHANNEL_COUNT {
            let frame_receiver_clone = frame_receiver.clone();
            let data_sender_clone = data_sender.clone();
            std::thread::spawn(move || {
                for frame_result in frame_receiver_clone {
                    let data_result =
                        frame_result.and_then(|frame| frame.to_bytes().map(|bytes| (frame, bytes)));

                    if data_sender_clone.send(data_result).is_err() {
                        break;
                    }
                }
            });
        }
    }

    fn spawn_io_worker(
        child: Child,
        stdin: ChildStdin,
        data_receiver: Receiver<Result<(Frame, Vec<u8>)>>,
        final_frame_sender: Sender<Result<Frame>>,
    ) {
        std::thread::spawn(move || {
            let pipeline = IoPipeline::new(child, stdin, final_frame_sender.clone());
            if let Err(e) = pipeline.run(data_receiver) {
                let _ = final_frame_sender.send(Err(e));
            }
        });
    }

    fn spawn_ffmpeg_process(video: &Video) -> Result<Child> {
        let scale_filter = format!("scale={}x{}:flags=lanczos", video.width, video.height);
        let input_dims = format!("{}x{}", video.get_scaled_width(), video.get_scaled_height());

        Command::new("ffmpeg")
            .args([
                "-hide_banner",
                "-loglevel",
                "error",
                "-y",
                "-i",
                video.input,
                "-thread_queue_size",
                "1024",
                "-f",
                "image2pipe",
                "-vcodec",
                "png",
                "-pix_fmt",
                "rgb24",
                "-s",
                &input_dims,
                "-framerate",
                &video.frame_rate.to_string(),
                "-i",
                "-",
                "-map",
                "0:a?",
                "-map",
                "0:s?",
                "-map",
                "1:v",
                "-map_metadata",
                "0",
                "-vf",
                &scale_filter,
                "-pix_fmt",
                "yuv420p",
                "-c:v",
                video.encoder,
                "-c:a",
                "copy",
                "-c:s",
                "copy",
                video.output,
            ])
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .spawn()
            .context("Failed to spawn FFmpeg process")
    }
}
