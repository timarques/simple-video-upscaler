use crate::frame::Frame;
use crate::error::Error;
use crate::video::Video;

use std::process::{Child, ChildStdout, Command, Stdio};
use std::io::Read;
use std::thread;

use crossbeam_channel::{bounded, Receiver, Sender};

pub struct Extract;

impl Extract {

    const PNG_FOOTER_SIGNATURE: &[u8] = &[0x00, 0x00, 0x00, 0x00, 0x49, 0x45, 0x4E, 0x44, 0xAE, 0x42, 0x60, 0x82];
    const CHUNK_SIZE: usize = 1024 * 100; // 100KB
    const MAX_FRAME_BUFFER_SIZE: usize = 1024 * 1024 * 10; // 10MB

    fn find_png_footer(data: &[u8]) -> Option<usize> {
        data.windows(12)
            .position(|window| window == Self::PNG_FOOTER_SIGNATURE)
            .map(|pos| pos + 12)
    }

    fn spawn_ffmpeg_process(video: &Video) -> Result<Child, Error> {
        Command::new("ffmpeg")
            .args(&[
                "-r", "1", 
                "-i", &video.input,
                "-pix_fmt", "rgb8",
                "-q:v", "1",
                "-vcodec", "png",
                "-f", "image2pipe",
                "-thread_queue_size", "1024",
                "pipe:1"
            ])
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .stdin(Stdio::null())
            .spawn()
            .map_err(|e| Error::new(format!("Failed to spawn ffmpeg process: {}", e)))
    }

    fn process_chunk(
        frame_buffer: &mut Vec<u8>,
        read_chunk: &mut Vec<u8>,
        stdout: &mut ChildStdout,
    ) -> Result<Option<Frame>, Error> {
        let size = stdout
            .read(read_chunk)
            .map_err(|e| Error::new(format!("Failed to read chunk: {}", e)))?;
        if size == 0 {
            return Ok(None);
        }
        frame_buffer.extend_from_slice(&read_chunk[..size]);
        if let Some(index) = Self::find_png_footer(&frame_buffer) {
            let bytes: Vec<u8> = frame_buffer.drain(..index).collect();
            let frame = Frame::from_bytes(&bytes)?;
            Ok(Some(frame))
        } else if frame_buffer.len() > Self::MAX_FRAME_BUFFER_SIZE {
            Err(Error::new(format!("Frame buffer is too large: {}", frame_buffer.len())))
        } else {
            Self::process_chunk(frame_buffer, read_chunk, stdout)
        }
    }

    fn process_stdout(mut stdout: ChildStdout, sender: Sender<Result<Frame, Error>>) {
        let mut frame_buffer = Vec::new();
        let mut read_chunk = vec![0u8; Self::CHUNK_SIZE];
        loop {
            match Self::process_chunk(&mut frame_buffer, &mut read_chunk, &mut stdout) {
                Ok(None) => {
                    break
                },
                Ok(Some(frame)) => {
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
    }

    pub fn execute(video: &Video) -> Result<Receiver<Result<Frame, Error>>, Error> {
        let (sender, receiver) = bounded(1);
        let mut child = Self::spawn_ffmpeg_process(&video)?;
        let stdout = child.stdout.take().unwrap();
        thread::spawn(move || {
            Self::process_stdout(stdout, sender);
            let _ = child.kill();
            let _ = child.wait();
        });

        Ok(receiver)
    }

}