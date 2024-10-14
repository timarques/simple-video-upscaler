use crate::frame::Frame;
use crate::error::Error;
use crate::video::Video;
use crate::model::Upscaler;

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;

use crossbeam_channel::{bounded, Receiver, Sender, TryRecvError};

pub struct Upscale;

impl Upscale {

    const MAX_JOBS: usize = 4;

    fn process_frame(
        mut frame: Frame,
        processing: &Arc<Mutex<Vec<usize>>>,
        upscaler: &Arc<dyn Upscaler>,
        shutdown_flag: &Arc<AtomicBool>,
        scale: u8,
    ) -> Result<Option<Frame>, Error> {
        processing.lock().expect("Failed to lock queue").push(frame.index);
        let width = frame.image.width();
        let height = frame.image.height();
        let frame_pixels = frame.image.to_rgb8().into_raw();
        let upscaled_pixels = upscaler.upscale(&frame_pixels, width as usize, height as usize)?;
        let upscaled_image = image::ImageBuffer::from_raw(
            width * scale as u32,
            height * scale as u32,
            upscaled_pixels
        ).unwrap();
        frame.image = image::DynamicImage::ImageRgb8(upscaled_image);
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
        upscaler: Arc<dyn Upscaler>,
        scale: u8,
    ) {
        let processing = Arc::new(Mutex::new(Vec::new()));
        let shutdown_flag = Arc::new(AtomicBool::new(false));
        while !shutdown_flag.load(Ordering::SeqCst) {
            match receiver.try_recv() {
                Ok(Ok(frame)) => {
                    match Self::process_frame(frame, &processing, &upscaler, &shutdown_flag, scale) {
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
        let model = video.model.as_ref().unwrap();
        let scale = model.get_scale();
        if scale == 1 {
            // to implement
        }
        let (sender, receiver) = bounded(1);
        let upscaler = model.create().unwrap();
        for _ in 0..Self::MAX_JOBS {
            let upscaler = upscaler.clone();
            let sender = sender.clone();
            let frames_receiver = frames_receiver.clone();
            thread::spawn(move || Self::process_incoming_frames(frames_receiver, sender, upscaler, scale));
        }
        receiver
    }

}