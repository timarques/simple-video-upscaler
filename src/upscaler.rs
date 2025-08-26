
use crate::model::Model;

use anyhow::{Result, anyhow};
use image::DynamicImage;
use std::sync::Arc;
use realcugan_rs::{RealCugan, Options as RealCuganOptions, OptionsModel as RealCuganOptionsModel};
use realesrgan_rs::{RealEsrgan, Options as RealEsrganOptions, OptionsModel as RealEsrganOptionsModel};

pub type UpscaleCallback = Arc<dyn Fn(&DynamicImage) -> Result<DynamicImage> + Send + Sync>;

struct UpscaleEngineWrapper<T> {
    inner: T,
}

impl<T> UpscaleEngineWrapper<T> {
    fn new(inner: T) -> Self {
        Self { inner }
    }
}

unsafe impl<T> Send for UpscaleEngineWrapper<T> {}
unsafe impl<T> Sync for UpscaleEngineWrapper<T> {}

#[derive(Clone)]
pub struct Upscaler {
    pub upscale_fn: UpscaleCallback,
}

impl Upscaler {
    fn new_realcugan(realcugan: RealCugan) -> Self {
        let wrapped_engine = Box::new(UpscaleEngineWrapper::new(realcugan));
        let upscale_fn = Arc::new(move |image: &DynamicImage| -> Result<DynamicImage> {
            wrapped_engine
                .inner
                .process_image(image)
                .map_err(|e| anyhow!("RealCugan upscale failed: {}", e))
        });
        
        Self { upscale_fn }
    }
    
    fn new_realesrgan(realesrgan: RealEsrgan) -> Self {
        let wrapped_engine = Box::new(UpscaleEngineWrapper::new(realesrgan));
        let upscale_fn = Arc::new(move |image: &DynamicImage| -> Result<DynamicImage> {
            wrapped_engine
                .inner
                .process_image(image)
                .map_err(|e| anyhow!("RealEsrgan upscale failed: {}", e))
        });
        
        Self { upscale_fn }
    }

    pub fn from_model(model: &Model) -> Result<Upscaler> {
        match model {
            &Model::RealCugan(scale) => Self::build_realcugan(scale),
            &Model::RealEsrAnime(scale) => Self::build_realesr_anime_video(scale),
            Model::RealEsrgan => Self::build_realesrgan_x4(),
            Model::RealEsrganAnime => Self::build_realesrgan_anime_x4(),
        }
    }

    fn build_realcugan(scale: u8) -> Result<Upscaler> {
        let model_type = match scale {
            2 => RealCuganOptionsModel::Se2xConservative,
            3 => RealCuganOptionsModel::Se3xConservative,
            4 => RealCuganOptionsModel::Se4xConservative,
            _ => return Err(anyhow!("Unsupported scale {scale} for RealCugan")),
        };

        let options = RealCuganOptions::default().model(model_type);
        let realcugan = RealCugan::new(options)?;
        Ok(Upscaler::new_realcugan(realcugan))
    }

    fn build_realesr_anime_video(scale: u8) -> Result<Upscaler> {
        let model_type = match scale {
            2 => RealEsrganOptionsModel::RealESRAnimeVideoV3x2,
            3 => RealEsrganOptionsModel::RealESRAnimeVideoV3x3,
            4 => RealEsrganOptionsModel::RealESRAnimeVideoV3x4,
            _ => return Err(anyhow!("Unsupported scale {scale} for RealESR AnimeVideo")),
        };

        let options = RealEsrganOptions::default().model(model_type);
        let realesrgan = RealEsrgan::new(options)?;
        Ok(Upscaler::new_realesrgan(realesrgan))
    }

    fn build_realesrgan_x4() -> Result<Upscaler> {
        let options = RealEsrganOptions::default()
            .model(RealEsrganOptionsModel::RealESRGANPlusx4);
        let realesrgan = RealEsrgan::new(options)?;
        Ok(Upscaler::new_realesrgan(realesrgan))
    }

    fn build_realesrgan_anime_x4() -> Result<Upscaler> {
        let options = RealEsrganOptions::default()
            .model(RealEsrganOptionsModel::RealESRGANPlusx4Anime);
        let realesrgan = RealEsrgan::new(options)?;
        Ok(Upscaler::new_realesrgan(realesrgan))
    }

    
    pub fn upscale(&self, image: &DynamicImage) -> Result<DynamicImage> {
        (self.upscale_fn)(image)
    }
}