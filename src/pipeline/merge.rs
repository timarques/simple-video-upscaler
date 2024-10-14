use crate::frame::Frame;
use crate::error::Error;
use crate::video::Video;

use std::io::Write;
use std::process::{Child, ChildStdin, Command, Stdio};
use crossbeam_channel::{Receiver, TryRecvError};

pub struct Merge;

impl Merge {

    fn spawn_ffmpeg_process(video: &Video) -> Result<Child, Error> {
        Command::new("ffmpeg")
            .args(&[
                "-i", &video.input,
                "-r", &video.frame_rate.to_string(),
                "-thread_queue_size", "1024",
                "-f", "image2pipe",
                "-vcodec", "png",
                "-i", "-",
                "-map", "0:a",
                "-map", "0:s?",
                "-map", "1:v",
                "-map_metadata", "0",
                "-vf", &format!("scale={}x{}:flags=lanczos", &video.width, &video.height),
                "-pix_fmt", "yuv420p",
                "-c:v", &video.encoder,
                "-c:a", "copy",
                "-c:s", "copy",
                "-y",
                &video.output
            ])
            .stdin(Stdio::piped())
            .stderr(Stdio::null())
            .stdout(Stdio::null())
            .spawn()
            .map_err(|e| Error::new(format!("Failed to spawn ffmpeg process: {}", e)))
    }

    fn process_stdin(mut stdin: ChildStdin, receiver: Receiver<Result<Frame, Error>>) -> Result<(), Error> {
        loop {
            match receiver.try_recv() {
                Ok(Ok(frame)) => {
                    let bytes = frame.to_bytes()?;
                    for _ in 0..(frame.duplicates + 1) {
                        stdin.write_all(&bytes).map_err(|e| Error::new(format!("Failed to write to stdin: {}", e)))?;
                    }
                }
                Ok(Err(e)) => return Err(e),
                Err(TryRecvError::Empty) => std::thread::yield_now(),
                Err(TryRecvError::Disconnected) => {
                    let _ = stdin.flush();
                    drop(stdin);
                    return Ok(())
                },
            }
        }
    }

    pub fn execute(video: &Video, receiver: Receiver<Result<Frame, Error>>) -> Result<(), Error> {
        let mut child = Self::spawn_ffmpeg_process(video)?;
        let stdin = child.stdin.take().unwrap();
        let result = Self::process_stdin(stdin, receiver);
        let _ = child.wait();
        if result.is_err() {
            let _ = child.kill();
        }
        result
    }

}