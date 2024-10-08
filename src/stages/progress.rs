use crate::error::Error;
use crate::frame::Frame;
use crate::args::Args;

use std::{fmt::Write, time::Instant};
use indicatif::{ProgressBar, ProgressState, ProgressStyle};
use crossbeam_channel::{bounded, Receiver, Sender, TryRecvError};

pub struct Progress {
    duplicates: usize,
    progress_bar: ProgressBar,
    start_time: Instant,
    frames_receiver: Receiver<Result<Frame, Error>>,
    sender: Sender<Result<Frame, Error>>,
}

impl Progress {

    fn new(
        args: &Args,
        frames_receiver: Receiver<Result<Frame, Error>>,
        sender: Sender<Result<Frame, Error>>
    ) -> Self {
        let progress_bar = ProgressBar::new(args.frame_count as u64);
        let progress_style = ProgressStyle::default_bar()
            .template("[{elapsed_precise}] [{eta_precise}] [{wide_bar:.white/green}] {pos}/{len} {percent} {msg}")
            .unwrap()
            .progress_chars("█▓▒░-")
            .with_key("eta", |state: &ProgressState, w: &mut dyn Write| write!(w, "{:.1}s", state.eta().as_secs_f64()).unwrap())
            .with_key("percent", |state: &ProgressState, w: &mut dyn Write| write!(w, "({:.0}%)", state.fraction() * 100.0).unwrap());
        progress_bar.set_style(progress_style);

        Self {
            duplicates: 0,
            progress_bar,
            start_time: Instant::now(),
            frames_receiver,
            sender,
        }
    }

    fn update_progress(&mut self, frame: &Frame) {
        let progress = frame.index + frame.duplicates;
        self.progress_bar.set_position(progress as u64);
        self.duplicates += frame.duplicates;

        let total_elapsed = self.start_time.elapsed();
        let avg_fps = progress as f64 / total_elapsed.as_secs_f64();
        let message = format!("[duplicates: {}] [fps: {:.0}]", self.duplicates, avg_fps);
        self.progress_bar.set_message(message);
    }

    fn start(mut self) {
        std::thread::spawn(move || {
            loop {
                match self.frames_receiver.try_recv() {
                    Ok(Ok(frame)) => {
                        self.update_progress(&frame);
                        if self.sender.send(Ok(frame)).is_err() {
                            break;
                        }
                    }
                    Ok(Err(e)) => {
                        let _ = self.sender.send(Err(e));
                        break;
                    }
                    Err(TryRecvError::Disconnected) => break,
                    Err(TryRecvError::Empty) => std::thread::yield_now(),
                }
            }
            self.progress_bar.finish();
        });
    }

    pub fn execute(args: &Args, frames_receiver: Receiver<Result<Frame, Error>>) -> Receiver<Result<Frame, Error>> {
        let (sender, receiver) = bounded(1);
        Self::new(args, frames_receiver, sender).start();
        receiver
    }

}