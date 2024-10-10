use crate::frame::Frame;
use crate::error::Error;

use crossbeam_channel::{bounded, Receiver, Sender, TryRecvError};
use std::thread;

pub struct FilterDuplicates;

impl FilterDuplicates {

    fn filter_frame(previous_frame: &mut Option<Frame>, frame: Frame) -> Option<Frame> {
        if let Some(mut previous) = previous_frame.take() {
            if previous.is_duplicate(&frame) {
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

    fn process_frames(frames_receiver: Receiver<Result<Frame, Error>>, sender: Sender<Result<Frame, Error>>) {
        let mut previous_frame = None;

        loop {
            match frames_receiver.try_recv() {
                Ok(Ok(frame)) => {
                    if let Some(filtered_frame) = Self::filter_frame(&mut previous_frame, frame) {
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

    pub fn execute(frames_receiver: Receiver<Result<Frame, Error>>) -> Receiver<Result<Frame, Error>> {
        let (sender, receiver) = bounded(1);
        thread::spawn(move || Self::process_frames(frames_receiver, sender));
        receiver
    }

}