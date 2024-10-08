use crate::frame::Frame;
use crate::error::Error;
use crate::args::Args;

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;

use crossbeam_channel::{bounded, Receiver, Sender, TryRecvError};
use realcugan_rs::RealCugan;

#[derive(Clone)]
pub struct Upscale {
    frames_receiver: Receiver<Result<Frame, Error>>,
    model: RealCugan,
    processing: Arc<Mutex<Vec<usize>>>,
    sender: Sender<Result<Frame, Error>>,
    shutdown_flag: Arc<AtomicBool>,
}

impl Upscale {
    const MAX_JOBS: usize = 4;

    fn new(args: &Args, frames_receiver: Receiver<Result<Frame, Error>>, sender: Sender<Result<Frame, Error>>) -> Self {
        let processing = Arc::new(Mutex::new(Vec::new()));
        Self {
            frames_receiver,
            model: args.model.clone(),
            processing,
            sender,
            shutdown_flag: Arc::new(AtomicBool::new(false)),
        }
    }

    fn process_frame(
        &self,
        mut frame: Frame,
    ) -> Result<Option<Frame>, Error> {
        self.processing.lock().expect("Failed to lock queue").push(frame.index);
        frame.image = self.model
            .process_image(frame.image)
            .map_err(|e| Error::UpscaleError(e.to_string()))?;
        
        loop {
            let min_index = {
                let queue = self.processing.lock().expect("Failed to lock queue");
                *queue.iter().min().unwrap_or(&0)
            };

            if min_index == frame.index {
                let mut queue = self.processing.lock().expect("Failed to lock queue");
                queue.retain(|&x| x != frame.index);
                return Ok(Some(frame));
            } else if self.shutdown_flag.load(Ordering::SeqCst) {
                return Ok(None);
            }
            std::thread::yield_now();
        }
    }

    fn process_incoming_frames(&self) {
        while !self.shutdown_flag.load(Ordering::SeqCst) {
            match self.frames_receiver.try_recv() {
                Ok(Ok(frame)) => {
                    match self.process_frame(frame) {
                        Ok(Some(processed_frame)) => {
                            if self.sender.send(Ok(processed_frame)).is_err() {
                                break;
                            }
                        }
                        Ok(None) => break,
                        Err(e) => {
                            let _ = self.sender.send(Err(e));
                            break;
                        }
                    }
                }
                Ok(Err(e)) => {
                    let _ = self.sender.send(Err(e));
                    break;
                }
                Err(TryRecvError::Empty) => std::thread::yield_now(),
                Err(TryRecvError::Disconnected) => break,
            }
        }
        self.shutdown_flag.store(true, Ordering::SeqCst);
    }

    fn start(&self) {
        for _ in 0..Self::MAX_JOBS {
            let worker = self.clone();
            thread::spawn(move || worker.process_incoming_frames());
        }
    }

    pub fn execute(args: &Args, frames_receiver: Receiver<Result<Frame, Error>>) -> Receiver<Result<Frame, Error>> {
        let (sender, receiver) = bounded(1);
        let this = Self::new(args, frames_receiver, sender);
        this.shutdown_flag.store(false, Ordering::SeqCst);
        this.start();
        receiver
    }
}