use crate::frame::Frame;
use crate::error::Error;
use crate::video::Video;

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;

use crossbeam_channel::{bounded, Receiver, Sender, TryRecvError};
use realcugan_rs::RealCugan;

pub struct Upscale;

impl Upscale {

    const MAX_JOBS: usize = 4;

    fn process_frame(
        mut frame: Frame,
        processing: &Arc<Mutex<Vec<usize>>>,
        model: &RealCugan,
        shutdown_flag: &Arc<AtomicBool>,
    ) -> Result<Option<Frame>, Error> {
        processing.lock().expect("Failed to lock queue").push(frame.index);
        frame.image = model
            .process_image(frame.image)
            .map_err(|e| Error::UpscaleError(e.to_string()))?;
        
        loop {
            let min_index = {
                let queue = processing.lock().expect("Failed to lock queue");
                *queue.iter().min().unwrap_or(&0)
            };

            if min_index == frame.index {
                let mut queue = processing.lock().expect("Failed to lock queue");
                queue.retain(|&x| x != frame.index);
                return Ok(Some(frame));
            } else if shutdown_flag.load(Ordering::SeqCst) {
                return Ok(None);
            }
            std::thread::yield_now();
        }
    }

    fn process_incoming_frames(
        receiver: Receiver<Result<Frame, Error>>,
        sender: Sender<Result<Frame, Error>>,
        model: RealCugan,
    ) {
        let processing = Arc::new(Mutex::new(Vec::new()));
        let shutdown_flag = Arc::new(AtomicBool::new(false));
        while !shutdown_flag.load(Ordering::SeqCst) {
            match receiver.try_recv() {
                Ok(Ok(frame)) => {
                    match Self::process_frame(frame, &processing, &model, &shutdown_flag) {
                        Ok(Some(processed_frame)) => {
                            if sender.send(Ok(processed_frame)).is_err() {
                                break;
                            }
                        }
                        Ok(None) => break,
                        Err(e) => {
                            let _ = sender.send(Err(e));
                            break;
                        }
                    }
                }
                Ok(Err(e)) => {
                    let _ = sender.send(Err(e));
                    break;
                }
                Err(TryRecvError::Empty) => std::thread::yield_now(),
                Err(TryRecvError::Disconnected) => break,
            }
        }
        shutdown_flag.store(true, Ordering::SeqCst);
    }

    pub fn execute(video: &Video, frames_receiver: Receiver<Result<Frame, Error>>) -> Receiver<Result<Frame, Error>> {
        let (sender, receiver) = bounded(1);
        let model = video.model.as_ref().unwrap();
        for _ in 0..Self::MAX_JOBS {
            let model = model.clone();
            let sender = sender.clone();
            let frames_receiver = frames_receiver.clone();
            thread::spawn(move || Self::process_incoming_frames(frames_receiver, sender, model));
        }
        receiver
    }

}