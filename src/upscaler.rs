use crate::model::Model;

use anyhow::{Result, anyhow};
use image::DynamicImage;
use upscaler::ImageBuffer;
use upscaler::realcugan::model::Model as RealCuganModel;
use upscaler::realcugan::options::{
    DenoiseLevel as RealCuganDenoiseLevel, Options as RealCuganOptions,
    ScaleFactor as RealCuganScaleFactor,
};
use upscaler::realesrgan::model::Model as RealEsrganModel;
use upscaler::realesrgan::options::{
    Options as RealEsrganOptions, ScaleFactor as RealEsrganScaleFactor,
};

pub struct Upscaler {
    inner: upscaler::Upscaler,
}

impl Upscaler {
    pub fn from_model(model: &Model) -> Result<Self> {
        let inner = match *model {
            Model::RealCugan(scale) => Self::build_realcugan(scale)?,
            Model::RealEsrAnime(scale) => Self::build_realesr_anime_video(scale)?,
            Model::RealEsrgan => Self::build_realesrgan_x4()?,
            Model::RealEsrganAnime => Self::build_realesrgan_anime_x4()?,
        };

        Ok(Self { inner })
    }

    fn build_realcugan(scale: u8) -> Result<upscaler::Upscaler> {
        let scale_factor = match scale {
            2 => RealCuganScaleFactor::X2,
            3 => RealCuganScaleFactor::X3,
            4 => RealCuganScaleFactor::X4,
            _ => return Err(anyhow!("Unsupported scale {scale} for RealCugan")),
        };

        let options = RealCuganOptions::builder()
            .with_model(RealCuganModel::Se)
            .with_scale_factor(scale_factor)
            .with_denoise_level(Some(RealCuganDenoiseLevel::Conservative))
            .build()?;

        Ok(upscaler::Upscaler::realcugan(options)?)
    }

    fn build_realesr_anime_video(scale: u8) -> Result<upscaler::Upscaler> {
        let scale_factor = match scale {
            2 => RealEsrganScaleFactor::X2,
            3 => RealEsrganScaleFactor::X3,
            4 => RealEsrganScaleFactor::X4,
            _ => return Err(anyhow!("Unsupported scale {scale} for RealESR AnimeVideo")),
        };

        let options = RealEsrganOptions::builder()
            .with_scale_factor(scale_factor)
            .with_model(RealEsrganModel::AnimeVideo)
            .build();

        Ok(upscaler::Upscaler::realesrgan(options)?)
    }

    fn build_realesrgan_x4() -> Result<upscaler::Upscaler> {
        let options = RealEsrganOptions::builder()
            .with_scale_factor(RealEsrganScaleFactor::X4)
            .with_model(RealEsrganModel::Plus)
            .build();

        Ok(upscaler::Upscaler::realesrgan(options)?)
    }

    fn build_realesrgan_anime_x4() -> Result<upscaler::Upscaler> {
        let options = RealEsrganOptions::builder()
            .with_scale_factor(RealEsrganScaleFactor::X4)
            .with_model(RealEsrganModel::PlusAnime)
            .build();

        Ok(upscaler::Upscaler::realesrgan(options)?)
    }

    pub fn upscale(&self, image: DynamicImage) -> Result<DynamicImage> {
        let input = ImageBuffer::from_image(image);
        let output = self
            .inner
            .upscale(&input)
            .map_err(|e| anyhow!("Upscale failed: {e}"))?;
        output
            .into_image()
            .map_err(|e| anyhow!("Failed to convert upscaled buffer to image: {e}"))
    }
}
