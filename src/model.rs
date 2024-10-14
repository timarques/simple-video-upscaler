use crate::error::Error;
use realcugan_rs::{RealCugan, Options as RealCuganOptions, OptionsModel as RealCuganOptionsModel};
use realesrgan_rs::{RealEsrgan, Options as RealEsrganOptions, OptionsModel as RealEsrganOptionsModel};

use std::sync::Arc;

#[derive(Clone, Copy, Debug)]
pub enum Model {
    RealCugan(u8),
    RealEsrAnime(u8),
    RealEsrgan,
    RealEsrganAnime,
}

impl Model {

    pub fn get_scale(&self) -> u8 {
        match self {
            Model::RealCugan(scale) => *scale,
            Model::RealEsrAnime(scale) => *scale,
            Model::RealEsrgan => 4,
            Model::RealEsrganAnime => 4,
        }
    }

    pub fn create(&self) -> Result<Arc<dyn Upscaler>, Error> {
        match self {
            Model::RealCugan(scale) => {
                let options = RealCuganOptions::default().model(match scale {
                    2 => RealCuganOptionsModel::Se2xConservative,
                    3 => RealCuganOptionsModel::Se3xConservative,
                    4 => RealCuganOptionsModel::Se4xConservative,
                    _ => unreachable!(),
                });
                RealCugan::new(options).map_err(|e| Error::ModelCreationError(e)).map(|r| Arc::new(r) as _)
            },
            Model::RealEsrAnime(scale) => {
                let options = RealEsrganOptions::default().model(match scale {
                    2 => RealEsrganOptionsModel::RealESRAnimeVideoV3x2,
                    3 => RealEsrganOptionsModel::RealESRAnimeVideoV3x3,
                    4 => RealEsrganOptionsModel::RealESRAnimeVideoV3x4,
                    _ => unreachable!(),
                });
                RealEsrgan::new(options).map_err(|e| Error::ModelCreationError(e)).map(|r| Arc::new(r) as _)
            },
            Model::RealEsrgan => {
                let options = RealEsrganOptions::default().model(RealEsrganOptionsModel::RealESRGANPlusx4);
                RealEsrgan::new(options).map_err(|e| Error::ModelCreationError(e)).map(|r| Arc::new(r) as _)
            },
            Model::RealEsrganAnime => {
                let options = RealEsrganOptions::default().model(RealEsrganOptionsModel::RealESRGANPlusx4Anime);
                RealEsrgan::new(options).map_err(|e| Error::ModelCreationError(e)).map(|r| Arc::new(r) as _)
            },
        }
    }
}

impl std::fmt::Display for Model {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Model::RealCugan(scale) => write!(f, "realcugan-x{}", scale),
            Model::RealEsrAnime(scale) => write!(f, "realesr-anime-x{}", scale),
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