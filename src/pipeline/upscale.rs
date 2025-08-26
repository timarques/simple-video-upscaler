use crate::{frame::Frame};
use crate::video::Video;
use crate::upscaler::Upscaler;
use super::CHANNEL_COUNT;

use anyhow::{Context, Result};
use crossbeam_channel::{bounded, Receiver, Sender};

pub struct UpscalePhase;

impl UpscalePhase {
    fn process_frame(frame: Frame, upscaler: &Upscaler) -> Result<Frame> {
        let image = upscaler.upscale(&frame.image)?;

        Ok(Frame {
            image,
            ..frame
        })
    }

    fn worker(
        frames_receiver: Receiver<Result<Frame>>,
        output_sender: Sender<Result<Frame>>,
        upscaler: &Upscaler,
    ) {
        for frame_result in frames_receiver.iter() {
            let result = frame_result
                .and_then(|frame| Self::process_frame(frame, upscaler));

            if output_sender.send(result).is_err() {
                break;
            }
        }
    }

    pub fn execute(
        video: &Video,
        frames_receiver: Receiver<Result<Frame>>,
    ) -> Result<Receiver<Result<Frame>>> {
        let model = video.model.as_ref().context("No upscaling model specified")?;

        let (output_sender, output_receiver) = bounded(CHANNEL_COUNT);

        for _ in 0..CHANNEL_COUNT {
            let frames_receiver = frames_receiver.clone();
            let output_sender = output_sender.clone();
            let upscaler = Upscaler::from_model(model)?;

            std::thread::spawn(move || Self::worker(frames_receiver, output_sender, &upscaler));
        }

        Ok(output_receiver)
    }
}