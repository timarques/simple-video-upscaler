use crate::frame::Frame;
use crate::error::Error;
use crate::args::Args;

use std::process::{Child, ChildStdout, Command, Stdio};
use std::io::Read;
use std::thread;
use std::sync::{Arc, atomic::{AtomicBool, Ordering}};

use crossbeam_channel::{bounded, Receiver, Sender};

#[derive(Clone)]
pub struct Extract {
    input_file: String,
    sender: Sender<Result<Frame, Error>>,
    is_closed: Arc<AtomicBool>,
}

impl Extract {
    const PNG_FOOTER_SIGNATURE: &'static [u8] = &[0x00, 0x00, 0x00, 0x00, 0x49, 0x45, 0x4E, 0x44, 0xAE, 0x42, 0x60, 0x82];
    const CHUNK_SIZE: usize = 1024 * 100; // 100KB
    const MAX_FRAME_BUFFER_SIZE: usize = 1024 * 1024 * 10; // 10MB

    fn new(args: &Args, sender: Sender<Result<Frame, Error>> ) -> Self {
        let is_closed = Arc::new(AtomicBool::new(false));
        Self { input_file: args.input.to_string(), sender, is_closed }
    }

    fn find_png_footer(data: &[u8]) -> Option<usize> {
        data.windows(12)
            .rposition(|window| window == Self::PNG_FOOTER_SIGNATURE)
            .map(|pos| pos + 12)
    }

    fn spawn_ffmpeg_process(&self) -> Result<Child, Error> {
        Command::new("ffmpeg")
            .args(&[
                "-r", "1",
                "-i", &self.input_file,
                "-thread_queue_size", "1024",
                "-threads", "1",
                "-q:v", "1",
                "-vcodec", "png",
                "-f", "image2pipe",
                "pipe:1"
            ])
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .stdin(Stdio::null())
            .spawn()
            .map_err(|_| Error::FfmpegFailed)
    }

    fn process_chunk(
        &self,
        frame_buffer: &mut Vec<u8>,
        read_chunk: &mut Vec<u8>,
        stdout: &mut ChildStdout,
    ) -> Result<Option<Frame>, Error> {
        let size = stdout.read(read_chunk)?;
        if size == 0 {
            return Ok(None);
        }
        frame_buffer.extend_from_slice(&read_chunk[..size]);
        if let Some(index) = Self::find_png_footer(&frame_buffer) {
            let bytes: Vec<u8> = frame_buffer.drain(..index).collect();
            let frame = Frame::from_bytes(&bytes)?;
            Ok(Some(frame))
        } else if frame_buffer.len() > Self::MAX_FRAME_BUFFER_SIZE {
            Err(Error::FrameBufferOverflow)
        } else {
            return self.process_chunk(frame_buffer, read_chunk, stdout)
        }
    }

    fn process_stdout(&self, mut stdout: ChildStdout) {
        let mut frame_buffer = Vec::new();
        let mut read_chunk = vec![0u8; Self::CHUNK_SIZE];

        loop {
            match self.process_chunk(&mut frame_buffer, &mut read_chunk, &mut stdout) {
                Ok(None) => {
                    break
                },
                Ok(Some(frame)) => {
                    if self.sender.send(Ok(frame)).is_err() {
                        break
                    }
                }
                Err(e) => {
                    let _ = self.sender.send(Err(e));
                    break
                }
            }
        }
    }

    fn start(self) -> Result<(), Error> {
        let mut child = self.spawn_ffmpeg_process()?;
        let stdout = child.stdout.take().unwrap();
        self.is_closed.store(false, Ordering::SeqCst);

        thread::spawn(move || {
            self.process_stdout(stdout);
            self.is_closed.store(true, Ordering::SeqCst);

            let _ = child.kill();
            let _ = child.wait();
        });

        Ok(())
    }

    pub fn execute(args: &Args) -> Result<Receiver<Result<Frame, Error>>, Error> {
        let (sender, receiver) = bounded(1);
        let this = Self::new(args, sender);
        this.start()?;
        Ok(receiver)
    }

}