use crate::frame::Frame;
use crate::error::Error;
use crate::video::Video;
use crate::model::Model;

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::collections::BTreeMap;

use crossbeam_channel::{bounded, Receiver, Sender};
use realcugan_rs::{RealCugan, Options as RealCuganOptions, OptionsModel as RealCuganOptionsModel};
use realesrgan_rs::{RealEsrgan, Options as RealEsrganOptions, OptionsModel as RealEsrganOptionsModel};

trait Upscaler: Sync + Send {
    fn upscale(&self, input: &[u8], width: usize, height: usize) -> Result<Vec<u8>, Error>;
}

impl Upscaler for RealCugan {
    fn upscale(&self, input: &[u8], width: usize, height: usize) -> Result<Vec<u8>, Error> {
        self.process(input, width, height).map_err(|e| Error::new(format!("RealCugan upscale failed: {}", e)))
    }
}

impl Upscaler for RealEsrgan {
    fn upscale(&self, input: &[u8], width: usize, height: usize) -> Result<Vec<u8>, Error> {
        self.process(input, width, height).map_err(|e| Error::new(format!("RealEsrgan upscale failed: {}", e)))
    }
}

pub struct Upscale;

impl Upscale {
    const MAX_JOBS: usize = 4;

    fn init_upscaler(model: &Model) -> Result<Arc<dyn Upscaler>, Error> {
        match model {
            Model::RealCugan(scale) => {
                let options = RealCuganOptions::default().model(match scale {
                    2 => RealCuganOptionsModel::Se2xConservative,
                    3 => RealCuganOptionsModel::Se3xConservative,
                    4 => RealCuganOptionsModel::Se4xConservative,
                    _ => return Err(Error::new(format!("Unsupported scale {} for RealCugan", scale))),
                });
                RealCugan::new(options)
                    .map_err(|e| Error::new(format!("Failed to initialize RealCugan upscaler: {}", e)))
                    .map(|r| Arc::new(r) as _)
            },
            Model::RealEsrAnime(scale) => {
                let options = RealEsrganOptions::default().model(match scale {
                    2 => RealEsrganOptionsModel::RealESRAnimeVideoV3x2,
                    3 => RealEsrganOptionsModel::RealESRAnimeVideoV3x3,
                    4 => RealEsrganOptionsModel::RealESRAnimeVideoV3x4,
                    _ => return Err(Error::new(format!("Unsupported scale {} for RealEsrAnime", scale))),
                });
                RealEsrgan::new(options)
                    .map_err(|e| Error::new(format!("Failed to initialize RealEsrAnime upscaler: {}", e)))
                    .map(|r| Arc::new(r) as _)
            },
            Model::RealEsrgan => {
                let options = RealEsrganOptions::default().model(RealEsrganOptionsModel::RealESRGANPlusx4);
                RealEsrgan::new(options)
                    .map_err(|e| Error::new(format!("Failed to initialize RealEsrgan upscaler: {}", e)))
                    .map(|r| Arc::new(r) as _)
            },
            Model::RealEsrganAnime => {
                let options = RealEsrganOptions::default().model(RealEsrganOptionsModel::RealESRGANPlusx4Anime);
                RealEsrgan::new(options)
                    .map_err(|e| Error::new(format!("Failed to initialize RealEsrganAnime upscaler: {}", e)))
                    .map(|r| Arc::new(r) as _)
            },
        }
    }

    fn process_frame(
        frame: Frame,
        upscaler: &Arc<dyn Upscaler>,
        scale: u8,
    ) -> Result<Frame, Error> {
        let width = frame.image.width();
        let height = frame.image.height();
        let frame_pixels = frame.image.to_rgb8().into_raw();
        let upscaled_pixels = upscaler.upscale(&frame_pixels, width as usize, height as usize)?;
        let upscaled_image = image::ImageBuffer::from_raw(
            width * scale as u32,
            height * scale as u32,
            upscaled_pixels
        ).unwrap();
        Ok(Frame {
            image: image::DynamicImage::ImageRgb8(upscaled_image),
            ..frame
        })
    }

    fn send_processed_frames(
        sender: &Sender<Result<Frame, Error>>,
        processed_frames: &mut BTreeMap<usize, Frame>,
        next_frame_index: &Arc<AtomicUsize>,
    ) {
        while let Some(frame) = processed_frames.remove(&next_frame_index.load(Ordering::SeqCst)) {
            let duplicates = frame.duplicates;
            if sender.send(Ok(frame)).is_err() {
                return;
            }
            next_frame_index.fetch_add(1 + duplicates, Ordering::SeqCst);
        }
    }

    fn process_incoming_frames(
        receiver: Receiver<Result<Frame, Error>>,
        sender: Sender<Result<Frame, Error>>,
        upscaler: Arc<dyn Upscaler>,
        scale: u8,
        next_frame_index: Arc<AtomicUsize>,
        processed_frames: Arc<Mutex<BTreeMap<usize, Frame>>>,
    ) { 
        while let Ok(frame_result) = receiver.recv() {
            let processed_frame = match frame_result {
                Ok(frame) => Self::process_frame(frame, &upscaler, scale),
                Err(e) => {
                    let _ = sender.send(Err(e));
                    return;
                }
            };
    
            match processed_frame {
                Ok(frame) => {
                    let mut processed_frames = processed_frames.lock().unwrap();
                    processed_frames.insert(frame.index, frame);
                    Self::send_processed_frames(&sender, &mut processed_frames, &next_frame_index);
                }
                Err(e) => {
                    let _ = sender.send(Err(e));
                    return;
                }
            }
        }
    }

    fn spawn_worker_threads(
        frames_receiver: Receiver<Result<Frame, Error>>,
        upscaler: Arc<dyn Upscaler>,
        scale: u8,
    ) -> Receiver<Result<Frame, Error>> {
        let (sender, receiver) = bounded(Self::MAX_JOBS);
        let next_frame_index = Arc::new(AtomicUsize::new(0));
        let processed_frames = Arc::new(Mutex::new(BTreeMap::new()));

        for _ in 0..Self::MAX_JOBS {
            let upscaler = upscaler.clone();
            let sender = sender.clone();
            let frames_receiver = frames_receiver.clone();
            let next_frame_index = next_frame_index.clone();
            let processed_frames = processed_frames.clone();

            thread::spawn(move || {
                Self::process_incoming_frames(
                    frames_receiver,
                    sender,
                    upscaler,
                    scale,
                    next_frame_index,
                    processed_frames,
                )
            });
        }

        receiver
    }

    pub fn execute(video: &Video, frames_receiver: Receiver<Result<Frame, Error>>) -> Result<Receiver<Result<Frame, Error>>, Error> {
        let model = video.model.as_ref().ok_or_else(|| Error::new("No upscaling model specified"))?;
        let scale = model.get_scale();
        if scale == 1 {
            return Err(Error::new("Upscale scale must be greater than 1"));
        }

        let upscaler = Self::init_upscaler(model)?;
        let receiver = Self::spawn_worker_threads(frames_receiver, upscaler, scale);
        Ok(receiver)
    }
}