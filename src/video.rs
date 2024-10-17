use crate::arguments::Arguments;
use crate::model::Model;
use crate::error::Error;

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
    pub duplicate_threshold: f64,
    pub scale: usize,
    original_width: usize,
    original_height: usize,
}

impl<'a> Video<'a> {
    pub fn new(arguments: &'a Arguments, input: &'a str, output: &'a str) -> Result<Self, Error> {
        let mut video = Self {
            width: 0,
            height: 0,
            original_width: 0,
            original_height: 0,
            frame_rate: 0.0,
            frame_count: 0,
            scale: 2,
            model: None,
            input,
            output,
            encoder: &arguments.encoder,
            duplicate_threshold: arguments.duplicate_threshold,
        };

        video.fetch_video_metadata()?;
        video.set_model_and_resolution(arguments);
        video.set_model(arguments);
        video.warn_if_resolution_adjusted(arguments);

        Ok(video)
    }

    pub fn get_scaled_width(&self) -> usize {
        self.original_width * self.scale
    }

    pub fn get_scaled_height(&self) -> usize {
        self.original_height * self.scale
    }

    fn parse_frame_rate(value: &str) -> Result<f64, Error> {
        let fps_parts: Vec<&str> = value.split('/').collect();
        if fps_parts.len() == 2 {
            let num = fps_parts[0].parse::<f64>()
                .map_err(|_| Error::new(format!("Failed to parse frame rate numerator: {}", fps_parts[0])))?;
            let den = fps_parts[1].parse::<f64>()
                .map_err(|_| Error::new(format!("Failed to parse frame rate denominator: {}", fps_parts[1])))?;
            Ok(num / den)
        } else {
            Err(Error::new(format!("Invalid frame rate format: {}", value)))
        }
    }

    fn fetch_video_metadata(&mut self) -> Result<(), Error> {
        let output = Command::new("ffprobe")
            .args(&[
                "-hide_banner", "-v", "error",
                "-select_streams", "v:0",
                "-count_frames",
                "-show_entries", "stream=nb_read_frames,r_frame_rate,width,height",
                "-of", "default=noprint_wrappers=1",
                self.input,
            ])
            .output()
            .map_err(|e| Error::new(format!("Failed to execute ffprobe: {}", e)))?;

        let data = String::from_utf8(output.stdout)
            .map_err(|e| Error::new(format!("Failed to parse ffprobe output: {}", e)))?;
        
        for line in data.lines() {
            if let Some((key, value)) = line.split_once('=') {
                match key {
                    "nb_read_frames" => self.frame_count = value.parse()
                        .map_err(|_| Error::new(format!("Failed to parse frame count: {}", value)))?,
                    "r_frame_rate" => self.frame_rate = Self::parse_frame_rate(value)?,
                    "width" => self.original_width = value.parse()
                        .map_err(|_| Error::new(format!("Failed to parse width: {}", value)))?,
                    "height" => self.original_height = value.parse()
                        .map_err(|_| Error::new(format!("Failed to parse height: {}", value)))?,
                    _ => {}
                }
            }
        }

        Ok(())
    }

    fn calculate_target_dimensions(&self, arguments: &Arguments, original_aspect_ratio: f64) -> (usize, usize) {
        match (arguments.width, arguments.height) {
            (Some(w), Some(h)) => (w, h),
            (Some(w), None) => (w, (w as f64 / original_aspect_ratio).round() as usize),
            (None, Some(h)) => ((h as f64 * original_aspect_ratio).round() as usize, h),
            (None, None) => (self.original_width * self.scale, self.original_height * self.scale),
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

        self.scale = if arguments.model != "realcugan" && arguments.model != "realesr-anime" {
            4
        } else {
            1 + (0..=3).rev()
                .find(|&scale| final_width > self.original_width * scale || final_height > self.original_height * scale)
                .unwrap_or(0)
        };

        self.width = final_width.min(final_width * self.scale);
        self.height = final_height.min(final_height * self.scale);
    }

    fn set_model(&mut self, arguments: &Arguments) {
        self.model = match (self.scale, arguments.model.as_str()) {
            (1, _) => None,
            (_, "realcugan") => Some(Model::RealCugan(self.scale as u8)),
            (_, "realesr-anime") => Some(Model::RealEsrAnime(self.scale as u8)),
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
                "Warning: Resolution adjusted from {}x{} to {}x{} to maintain aspect ratio and scaling factor.",
                requested_width, requested_height, self.width, self.height
            );
        }
    }
}