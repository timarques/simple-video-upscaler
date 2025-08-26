use crate::arguments::Arguments;
use crate::model::Model;

use anyhow::{Context, Result};
use std::process::Command;

#[derive(Clone)]
pub struct Video<'a> {
    pub width: usize,
    pub height: usize,
    pub frame_rate: f64,
    pub frame_count: usize,
    pub model: Option<Model>,
    pub input: &'a str,
    pub output: &'a str,
    pub encoder: &'a str,
    pub scale: f32,
    original_width: usize,
    original_height: usize,
}

impl<'a> Video<'a> {
    pub fn new(arguments: &'a Arguments, input: &'a str, output: &'a str) -> Result<Self> {
        let mut video = Self {
            width: 0,
            height: 0,
            original_width: 0,
            original_height: 0,
            frame_rate: 0.0,
            frame_count: 0,
            scale: arguments.scale.unwrap_or(2.0),
            model: None,
            input,
            output,
            encoder: &arguments.encoder
        };

        video.fetch_video_metadata()?;
        video.set_model_and_resolution(arguments);
        video.set_model(arguments);
        video.warn_if_resolution_adjusted(arguments);

        Ok(video)
    }

    pub fn get_scaled_width(&self) -> usize {
        (self.original_width as f32 * self.scale).round() as usize
    }

    pub fn get_scaled_height(&self) -> usize {
        (self.original_height as f32 * self.scale).round() as usize
    }

    fn parse_frame_rate(value: &str) -> Result<f64> {
        let frame_rate_parts: Vec<&str> = value.split('/').collect();
        
        if let [numerator_str, denominator_str] = frame_rate_parts.as_slice() {
            let numerator = numerator_str.parse::<f64>()
                .with_context(|| format!("Failed to parse frame rate numerator: {numerator_str}"))?;
            let denominator = denominator_str.parse::<f64>()
                .with_context(|| format!("Failed to parse frame rate denominator: {denominator_str}"))?;
            Ok(numerator / denominator)
        } else {
            anyhow::bail!("Invalid frame rate format: {value}")
        }
    }

    fn fetch_video_metadata(&mut self) -> Result<()> {
        let output = Command::new("ffprobe")
            .args([
                "-hide_banner", "-v", "error",
                "-select_streams", "v:0",
                "-count_frames",
                "-show_entries", "stream=nb_read_frames,r_frame_rate,width,height",
                "-of", "default=noprint_wrappers=1",
                self.input,
            ])
            .output()
            .with_context(|| "Failed to execute ffprobe")?;

        let data = String::from_utf8(output.stdout)
            .with_context(|| "Failed to parse ffprobe output")?;
        
        for line in data.lines() {
            if let Some((key, value)) = line.split_once('=') {
                match key {
                    "r_frame_rate" => self.frame_rate = Self::parse_frame_rate(value)?,
                    "nb_read_frames" => self.frame_count = {
                        value
                            .parse()
                            .with_context(|| format!("Failed to parse frame count: {value}"))?
                    },
                    "width" => self.original_width = {
                        value
                            .parse()
                            .with_context(|| format!("Failed to parse width: {value}"))?
                    },
                    "height" => self.original_height = {
                        value
                            .parse()
                            .with_context(|| format!("Failed to parse height: {value}"))?
                    },
                    _ => {}
                }
            }
        }

        Ok(())
    }

    fn calculate_target_dimensions(&self, arguments: &Arguments, original_aspect_ratio: f64) -> (usize, usize) {
        match (arguments.width, arguments.height) {
            (Some(width), Some(height)) => (width, height),
            (Some(width), None) => (width, (width as f64 / original_aspect_ratio).round() as usize),
            (None, Some(height)) => ((height as f64 * original_aspect_ratio).round() as usize, height),
            (None, None) => {
                if arguments.scale.is_some() {
                    (self.get_scaled_width(), self.get_scaled_height())
                } else {
                    let calculated_scale = self.calculate_optimal_scale(arguments);
                    ((self.original_width as f32 * calculated_scale).round() as usize,
                     (self.original_height as f32 * calculated_scale).round() as usize)
                }
            }
        }
    }

    fn calculate_optimal_scale(&self, arguments: &Arguments) -> f32 {
        match arguments.model.as_str() {
            "realcugan" | "realesr-anime" => {
                [1.0, 2.0, 3.0, 4.0].iter()
                    .rev()
                    .find(|&&scale| {
                        let scaled_width = (self.original_width as f32 * scale).round() as usize;
                        let scaled_height = (self.original_height as f32 * scale).round() as usize;
                        scaled_width <= 7680 && scaled_height <= 4320
                    })
                    .copied()
                    .unwrap_or(1.0)
            }
            "realesrgan" | "realesrgan-anime" => 4.0,
            _ => 2.0
        }
    }

    fn adjust_for_aspect_ratio(&self, width: usize, height: usize, original_aspect_ratio: f64) -> (usize, usize) {
        let target_aspect_ratio = width as f64 / height as f64;

        if (target_aspect_ratio - original_aspect_ratio).abs() <= 0.01 {
            return (width, height);
        }

        if target_aspect_ratio > original_aspect_ratio {
            let new_width = (height as f64 * original_aspect_ratio).round() as usize;
            (new_width, height)
        } else {
            let new_height = (width as f64 / original_aspect_ratio).round() as usize;
            (width, new_height)
        }
    }

    fn set_model_and_resolution(&mut self, arguments: &Arguments) { 
        let original_aspect_ratio = self.original_width as f64 / self.original_height as f64;
        let (target_width, target_height) = self.calculate_target_dimensions(arguments, original_aspect_ratio);
        let (final_width, final_height) = self.adjust_for_aspect_ratio(target_width, target_height, original_aspect_ratio);

        if arguments.scale.is_none() {
            self.scale = self.calculate_optimal_scale(arguments);
        }

        self.width = final_width;
        self.height = final_height;
    }

    fn set_model(&mut self, arguments: &Arguments) {
        let scale_integer = self.scale.round() as u8;
        
        self.model = match (scale_integer, arguments.model.as_str()) {
            (1, _) => None,
            (_, "realcugan") => Some(Model::RealCugan(scale_integer)),
            (_, "realesr-anime") => Some(Model::RealEsrAnime(scale_integer)),
            (_, "realesrgan") => Some(Model::RealEsrgan),
            (_, "realesrgan-anime") => Some(Model::RealEsrganAnime),
            _ => None,
        };
    }

    fn warn_if_resolution_adjusted(&self, arguments: &Arguments) {
        let requested_width = arguments.width.unwrap_or(0);
        let requested_height = arguments.height.unwrap_or(0);

        if (requested_width > 0 && self.width != requested_width) || 
           (requested_height > 0 && self.height != requested_height) {
            println!(
                "Warning: Resolution adjusted from {requested_width}x{requested_height} to {}x{} to maintain aspect ratio and scaling factor.",
                self.width, self.height
            );
        }
    }
}