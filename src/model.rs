use crate::error::Error;
use realcugan_rs::{RealCugan, Options as RealCuganOptions, OptionsModel as RealCuganOptionsModel};
use realesrgan_rs::{RealEsrgan, Options as RealEsrganOptions, OptionsModel as RealEsrganOptionsModel};

use std::sync::Arc;

#[derive(Clone, Copy, Debug)]
pub enum Model {
    RealCugan(usize),
    RealEsrgan,
    RealEsrganAnime,
}

impl Model {
    pub fn create(&self) -> Arc<dyn Upscaler> {
        match self {
            Model::RealCugan(scale) => {
                let model = match scale {
                    2 => RealCuganOptionsModel::Se2xConservative,
                    3 => RealCuganOptionsModel::Se3xConservative,
                    4 => RealCuganOptionsModel::Se4xConservative,
                    _ => unreachable!(),
                };
                let options = RealCuganOptions::default().model(model);
                Arc::new(RealCugan::new(options).unwrap())
            },
            Model::RealEsrgan => {
                let options = RealEsrganOptions::default().model(RealEsrganOptionsModel::RealESRGANPlusx4);
                Arc::new(RealEsrgan::new(options).unwrap())
            },
            Model::RealEsrganAnime => {
                let options = RealEsrganOptions::default().model(RealEsrganOptionsModel::RealESRGANPlusx4Anime);
                Arc::new(RealEsrgan::new(options).unwrap())
            },
        }
    }
}

impl std::fmt::Display for Model {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Model::RealCugan(scale) => write!(f, "realcugan-x{}", scale),
            Model::RealEsrgan => write!(f, "realesrgan-x4"),
            Model::RealEsrganAnime => write!(f, "realesrgan-anime-x4"),
        }
    }
}

pub trait Upscaler: Sync + Send {
    fn upscale(&self, input: &[u8], width: usize, height: usize) -> Result<Vec<u8>, Error>;
}

impl Upscaler for RealCugan {
    fn upscale(&self, input: &[u8], width: usize, height: usize) -> Result<Vec<u8>, Error> {
        self.process(input, width, height).map_err(|e| Error::UpscaleError(e))
    }
}

impl Upscaler for RealEsrgan {
    fn upscale(&self, input: &[u8], width: usize, height: usize) -> Result<Vec<u8>, Error> {
        self.process(input, width, height).map_err(|e| Error::UpscaleError(e))
    }
}