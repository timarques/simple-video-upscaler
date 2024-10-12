use crate::error::Error;
use crate::frame::Frame;
use crate::video::Video;

use std::fmt::Write;
use std::time::Instant;
use indicatif::{ProgressBar, ProgressState, ProgressStyle};
use crossbeam_channel::{bounded, Receiver, Sender, TryRecvError};

pub struct Progress;

impl Progress {

    pub fn create_progress_bar(video: &Video) -> ProgressBar {
        let progress_bar = ProgressBar::new(video.frame_count as u64);
        let progress_template = "[{elapsed_precise}] [{eta_precise}] [{wide_bar:.white/green}] {pos}/{len} {percent} {msg}";
        let file_template = format!("{} -> {}", video.input, video.output);
        let options_template = format!(
            "[resolutin: {}x{}] [model: {}] [encoder: {}]", 
            video.width,
            video.height,
            video.model.unwrap().to_string(),
            video.encoder
        );
        let progress_style = ProgressStyle::default_bar()
            .template(&format!("{}\n{}\n{}", file_template, options_template, progress_template))
            .unwrap()
            .progress_chars("█▓▒░-")
            .with_key("eta", |state: &ProgressState, w: &mut dyn Write| write!(w, "{:.1}s", state.eta().as_secs_f64()).unwrap())
            .with_key("percent", |state: &ProgressState, w: &mut dyn Write| write!(w, "({:.0}%)", state.fraction() * 100.0).unwrap());
        progress_bar.set_style(progress_style);
        progress_bar
    }

    fn update_progress(progress_bar: &ProgressBar, position: usize, duplicates: usize, frame_rate: f64) {
        progress_bar.set_position(position as u64);
        progress_bar.set_message(format!("[duplicates: {}] [fps: {:.0}]", duplicates, frame_rate));
    }

    fn process_incoming_frames(
        receiver: Receiver<Result<Frame, Error>>,
        sender: Sender<Result<Frame, Error>>,
        progress_bar: ProgressBar,
    ) {
        let start_time = Instant::now();
        let mut duplicates = 0;
        loop {
            match receiver.try_recv() {
                Ok(Ok(frame)) => {
                    let position = frame.index + frame.duplicates;
                    duplicates += frame.duplicates;
                    if sender.send(Ok(frame)).is_err() {
                        break;
                    }
                    let total_elapsed = start_time.elapsed();
                    let frame_rate = position as f64 / total_elapsed.as_secs_f64();
                    Self::update_progress(&progress_bar, position, duplicates, frame_rate);
                }
                Ok(Err(e)) => {
                    let _ = sender.send(Err(e));
                    break;
                }
                Err(TryRecvError::Disconnected) => break,
                Err(TryRecvError::Empty) => std::thread::yield_now(),
            }
        }
        progress_bar.finish();
    }

    pub fn execute(video: &Video, frames_receiver: Receiver<Result<Frame, Error>>) -> Receiver<Result<Frame, Error>> {
        let (sender, receiver) = bounded(1);
        let progress_bar = Self::create_progress_bar(video);
        Self::update_progress(&progress_bar, 0, 0, 0.0);
        std::thread::spawn(move || Self::process_incoming_frames(frames_receiver, sender, progress_bar));
        receiver
    }

}