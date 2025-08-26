use crate::frame::Frame;
use crate::video::Video;

use anyhow::Result;
use std::fmt::Write;
use std::time::Instant;
use indicatif::{ProgressBar, ProgressState, ProgressStyle};
use crossbeam_channel::{Receiver, TryRecvError};

pub struct ProgressPhase;

impl ProgressPhase {

    pub fn create_progress_bar(video: &Video) -> ProgressBar {
        let progress_template = "[{elapsed_precise}] [{eta_precise}] [{wide_bar:.white/green}] {pos}/{len} {percent} {msg}";
        let file_template = format!("{input} -> {output}", input = video.input, output = video.output);
        let options_template = format!(
            "[resolution: {width}x{height}] [model: {model}] [encoder: {encoder}]", 
            width = video.width,
            height = video.height,
            model = video.model.unwrap(),
            encoder = video.encoder
        );

        let progress_style = ProgressStyle::default_bar()
            .template(&format!("{file_template}\n{options_template}\n{progress_template}"))
            .unwrap()
            .progress_chars("█▓▒░-")
            .with_key("eta", |state: &ProgressState, w: &mut dyn Write| write!(w, "{:.1}s", state.eta().as_secs_f64()).unwrap())
            .with_key("percent", |state: &ProgressState, w: &mut dyn Write| write!(w, "({:.0}%)", state.fraction() * 100.0).unwrap());
        
        let progress_bar = ProgressBar::new(video.frame_count as u64);
        progress_bar.set_style(progress_style);
        progress_bar
    }

    fn update_progress(progress_bar: &ProgressBar, position: usize, frame_rate: f64) {
        progress_bar.set_position(position as u64);
        progress_bar.set_message(format!("[frames_per_second: {frame_rate:.0}]"));
    }

    fn process_incoming_frames(
        receiver: Receiver<Result<Frame>>,
        progress_bar: ProgressBar,
    ) {
        let start_time = Instant::now();
        loop {
            match receiver.try_recv() {
                Ok(Ok(frame)) => {
                    let position = frame.index;
                    let total_elapsed = start_time.elapsed();
                    let frame_rate = position as f64 / total_elapsed.as_secs_f64();
                    Self::update_progress(&progress_bar, position, frame_rate);
                }
                Ok(Err(_)) => break,
                Err(TryRecvError::Disconnected) => break,
                Err(TryRecvError::Empty) => std::thread::yield_now(),
            }
        }
        progress_bar.finish();
    }

    pub fn execute(video: &Video, frames_receiver: Receiver<Result<Frame>>) {
        let progress_bar = Self::create_progress_bar(video);
        Self::update_progress(&progress_bar, 0, 0.0);
        Self::process_incoming_frames(frames_receiver, progress_bar);
    }

}