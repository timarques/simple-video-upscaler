use crate::frame::Frame;
use crate::video::Video;
use super::CHANNEL_COUNT;

use anyhow::{anyhow, bail, Context, Result};
use std::process::{Child, ChildStdout, Command, Stdio};
use std::io::{Read, BufReader};
use std::thread;

use crossbeam_channel::{bounded, Receiver, Sender};

const PNG_FOOTER_SIGNATURE: &[u8] = &[0x00, 0x00, 0x00, 0x00, 0x49, 0x45, 0x4E, 0x44, 0xAE, 0x42, 0x60, 0x82];
const CHUNK_SIZE: usize = 1024 * 100; // 100KB
const MAX_FRAME_BUFFER_SIZE: usize = 1024 * 1024 * 10; // 10MB

pub struct ExtractPhase;

impl ExtractPhase {    

    fn find_png_footer(data: &[u8]) -> Option<usize> {
        data.windows(12)
            .position(|window| window == PNG_FOOTER_SIGNATURE)
            .map(|pos| pos + 12)
    }

    fn spawn_ffmpeg_process(video: &Video) -> Result<Child> {
        Command::new("ffmpeg")
            .args([
                "-r", "1",
                "-i", video.input,
                "-pix_fmt", "rgb8",
                "-q:v:0", "1",
                "-vcodec", "png",
                "-f", "image2pipe",
                "-thread_queue_size", "100",
                "pipe:1"
            ])
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .stdin(Stdio::null())
            .spawn()
            .map_err(|error| anyhow!("Failed to spawn ffmpeg process: {error}"))
    }

    fn process_chunk(
        frame_buffer: &mut Vec<u8>,
        read_chunk: &mut Vec<u8>,
        buff_reader: &mut BufReader<&mut ChildStdout>,
        frame_count: usize
    ) -> Result<Option<Frame>> {
        let size = buff_reader
            .read(read_chunk)
            .map_err(|error| anyhow!("Failed to read chunk: {error}"))?;
        
        if size == 0 {
            return Ok(None);
        }

        frame_buffer.extend_from_slice(&read_chunk[..size]);

        if let Some(index) = Self::find_png_footer(frame_buffer) {
            let bytes: Vec<u8> = frame_buffer.drain(..index).collect();
            let frame = Frame::from_bytes(frame_count, &bytes)?;
            Ok(Some(frame))
        } else if frame_buffer.len() > MAX_FRAME_BUFFER_SIZE {
            bail!("Frame buffer is too large: {size}", size = frame_buffer.len())
        } else {
            Self::process_chunk(frame_buffer, read_chunk, buff_reader, frame_count)
        }
    }

    fn process_stdout(mut stdout: ChildStdout, sender: Sender<Result<Frame>>) {
        let mut frame_buffer = Vec::new();
        let mut read_chunk = vec![0u8; CHUNK_SIZE];
        let mut buff_reader = BufReader::new(&mut stdout);
        let mut frame_count = 0;
        loop {
            match Self::process_chunk(&mut frame_buffer, &mut read_chunk, &mut buff_reader, frame_count) {
                Ok(None) => {
                    break
                },
                Ok(Some(frame)) => {
                    frame_count += 1;
                    if sender.send(Ok(frame)).is_err() {
                        break
                    }
                }
                Err(e) => {
                    let _ = sender.send(Err(e));
                    break
                }
            }
        }
        drop(stdout);
    }

    pub fn execute(video: &Video) -> Result<Receiver<Result<Frame>>> {
        let (sender, receiver) = bounded(CHANNEL_COUNT);
        let mut child = Self::spawn_ffmpeg_process(video)?;
        let stdout = child.stdout.take().context("Failed to get stdout handle")?;
        thread::spawn(move || {
            Self::process_stdout(stdout, sender);
            let _ = child.kill();
            let _ = child.wait();
        });

        Ok(receiver)
    }

}