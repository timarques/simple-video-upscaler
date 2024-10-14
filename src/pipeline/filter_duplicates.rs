use crate::{frame::Frame, video::Video};
use crate::error::Error;

use crossbeam_channel::{bounded, Receiver, Sender, TryRecvError};
use std::thread;

pub struct FilterDuplicates;

impl FilterDuplicates {

    fn frame_is_duplicate(frame1: &Frame, frame2: &Frame, threshold: f64) -> bool {
        let result = image_compare::rgb_hybrid_compare(
            &frame1.image.to_rgb8(), 
            &frame2.image.to_rgb8()
        );
        if let Ok(result) = result {
            if result.score >= threshold {
                return true
            }
        }
        false
    }

    fn filter_frame(previous_frame: &mut Option<Frame>, frame: Frame, threshold: f64) -> Option<Frame> {
        if let Some(mut previous) = previous_frame.take() {
            if Self::frame_is_duplicate(&previous, &frame, threshold) {
                previous.add_duplicate();
                *previous_frame = Some(previous);
                None
            } else {
                *previous_frame = Some(frame);
                Some(previous)
            }
        } else {
            *previous_frame = Some(frame);
            None
        }
    }

    fn process_frames(
        frames_receiver: Receiver<Result<Frame, Error>>,
        sender: Sender<Result<Frame, Error>>,
        threshold: f64
    ) {
        let mut previous_frame = None;

        loop {
            match frames_receiver.try_recv() {
                Ok(Ok(frame)) => {
                    if let Some(filtered_frame) = Self::filter_frame(&mut previous_frame, frame, threshold) {
                        if sender.send(Ok(filtered_frame)).is_err() {
                            break;
                        }
                    }
                }
                Ok(Err(e)) => {
                    let _ = sender.send(Err(e));
                    break;
                }
                Err(TryRecvError::Disconnected) => {
                    if let Some(previous) = previous_frame.take() {
                        let _ = sender.send(Ok(previous));
                    }
                    break;
                },
                Err(TryRecvError::Empty) => { thread::yield_now(); }
            }
        }
    }

    pub fn execute(video: &Video, frames_receiver: Receiver<Result<Frame, Error>>) -> Receiver<Result<Frame, Error>> {
        let (sender, receiver) = bounded(1);
        let threshold = video.duplicate_threshold;
        thread::spawn(move || Self::process_frames(frames_receiver, sender, threshold));
        receiver
    }

}