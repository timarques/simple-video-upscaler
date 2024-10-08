use crate::frame::Frame;
use crate::error::Error;
use crate::args::Args;

use std::io::Write;
use std::process::{Child, ChildStdin, Command, Stdio};
use crossbeam_channel::{Receiver, TryRecvError};

pub struct Merge {
    input_file: String,
    outpu_file: String,
    frame_rate: f64,
    encoder: String,
    width: usize,
    height: usize,
    receiver: Receiver<Result<Frame, Error>>,
}

impl Merge {

    fn new(args: &Args, receiver: Receiver<Result<Frame, Error>>) -> Self {
        Self {
            input_file: args.input.to_string(),
            outpu_file: args.output.to_string(),
            frame_rate: args.frame_rate,
            encoder: args.encoder.to_string(),
            width: args.width,
            height: args.height,
            receiver,
        }
    }

    fn spawn_ffmpeg_process(&self) -> Result<Child, Error> {
        Command::new("ffmpeg")
            .args(&[
                "-thread_queue_size", "1024",
                "-i", &self.input_file,
                "-threads", "1",
                "-r", &self.frame_rate.to_string(),
                "-f", "image2pipe",
                "-vcodec", "png",
                "-i", "-",
                "-threads", "1",
                "-map_metadata", "0",
                "-map", "1:v",
                "-map", "0:a",
                "-map", "0:s?",
                "-vf", &format!("scale={}x{}", &self.width, &self.height),
                "-c:v", &self.encoder,
                "-y",
                &self.outpu_file
            ])
            .stdin(Stdio::piped())
            .stderr(Stdio::null())
            .stdout(Stdio::null())
            .spawn()
            .map_err(Error::Io)
    }

    fn process_stdin(&self, mut stdin: ChildStdin) -> Result<(), Error> {
        loop {
            match self.receiver.try_recv() {
                Ok(Ok(frame)) => {
                    let bytes = frame.to_bytes()?;
                    for _ in 0..frame.duplicates {
                        stdin.write_all(&bytes).map_err(Error::Io)?;
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

    fn start(&self) -> Result<(), Error> {
        let mut child = self.spawn_ffmpeg_process()?;
        let stdin = child.stdin.take().unwrap();
        let result = self.process_stdin(stdin);
        let _ = child.kill();
        let _ = child.wait();
        result
    }

    pub fn execute(args: &Args, receiver: Receiver<Result<Frame, Error>>) -> Result<(), Error> {
        let this = Self::new(args, receiver);
        this.start()?;
        Ok(())
    }

}