use crate::frame::Frame;
use crate::error::Error;

use crossbeam_channel::{unbounded, Receiver, Sender, TryRecvError};

pub struct FilterDuplicates {
    previous_frame: Option<Frame>,
    frames_receiver: Receiver<Result<Frame, Error>>,
    sender: Sender<Result<Frame, Error>>,
}

impl FilterDuplicates {

    fn new(frames_receiver: Receiver<Result<Frame, Error>>, sender: Sender<Result<Frame, Error>>) -> Self {
        Self {
            previous_frame: None,
            frames_receiver,
            sender,
        }
    }

    fn filter_frame(&mut self, frame: Frame) -> Option<Frame> {
        if let Some(mut previous) = self.previous_frame.take() {
            if previous.is_duplicate(&frame) {
                previous.add_duplicate();
                self.previous_frame = Some(previous);
                None
            } else {
                self.previous_frame = Some(frame);
                Some(previous)
            }
        } else {
            self.previous_frame = Some(frame);
            None
        }
    }

    fn start(mut self) {
        std::thread::spawn(move || {
            loop {
                match self.frames_receiver.try_recv() {
                    Ok(Ok(frame)) => {
                        if let Some(filtered_frame) = self.filter_frame(frame) {
                            if self.sender.send(Ok(filtered_frame)).is_err() {
                                break
                            }
                        }
                    }
                    Ok(Err(e)) => {
                        let _ = self.sender.send(Err(e));
                        break
                    }
                    Err(TryRecvError::Disconnected) => {
                        if let Some(previous) = self.previous_frame.take() {
                            let _ = self.sender.send(Ok(previous));
                        }
                        break
                    },
                    Err(TryRecvError::Empty) => { std::thread::yield_now(); }
                }
            }
        });
    }

    pub fn execute(frames_receiver: Receiver<Result<Frame, Error>>) -> Receiver<Result<Frame, Error>> {
        let (sender, receiver) = unbounded();
        Self::new(frames_receiver, sender).start();
        return receiver
    }


}