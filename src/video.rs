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
}

impl<'a> Video<'a> {
    pub fn new(arguments: &'a Arguments, input: &'a str, output: &'a str) -> Result<Self, Error> {
        let mut video = Self {
            width: 0,
            height: 0,
            frame_rate: 0.0,
            frame_count: 0,
            model: None,
            input,
            output,
            encoder: &arguments.encoder,
        };

        video.fetch_video_metadata()?;
        video.set_model_and_resolution(arguments);

        Ok(video)
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
            .output()?;

        let data = String::from_utf8_lossy(&output.stdout);
        
        for line in data.lines() {
            if let Some((key, value)) = line.split_once('=') {
                match key {
                    "nb_read_frames" => self.frame_count = value.parse().unwrap_or(0),
                    "r_frame_rate" => self.frame_rate = Self::parse_frame_rate(value),
                    "width" => self.width = value.parse().unwrap_or(0),
                    "height" => self.height = value.parse().unwrap_or(0),
                    _ => {}
                }
            }
        }

        Ok(())
    }

    fn parse_frame_rate(value: &str) -> f64 {
        let fps_parts: Vec<&str> = value.split('/').collect();
        if fps_parts.len() == 2 {
            let num = fps_parts[0].parse::<f64>().unwrap_or(0.0);
            let den = fps_parts[1].parse::<f64>().unwrap_or(1.0);
            num / den
        } else {
            0.0
        }
    }

    fn set_model_and_resolution(&mut self, arguments: &Arguments) {
        let original_aspect_ratio = self.width as f64 / self.height as f64;
        let (target_width, target_height) = self.calculate_target_dimensions(arguments, original_aspect_ratio);
        let (final_width, final_height) = self.adjust_for_aspect_ratio(target_width, target_height, original_aspect_ratio);

        let scale = if arguments.model != "realcugan" {
            4
        } else {
            1 + (0..=3).rev()
            .find(|&scale| final_width >= self.width * scale || final_height >= self.height * scale)
            .unwrap_or(1)
        };

        self.width = final_width.min(final_width * scale);
        self.height = final_height.min(final_height * scale);

        if scale < 2 {
            self.model = None;
        } else if arguments.model == "realcugan" {
            self.model = Some(Model::RealCugan(scale));
        } else if arguments.model == "realesrgan" {
            self.model = Some(Model::RealEsrgan);
        } else if arguments.model == "realesrgan-anime" {
            self.model = Some(Model::RealEsrganAnime);
        }

        self.warn_if_resolution_adjusted(arguments, final_width, final_height);
    }

    fn calculate_target_dimensions(&self, arguments: &Arguments, original_aspect_ratio: f64) -> (usize, usize) {
        match (arguments.width, arguments.height) {
            (Some(w), Some(h)) => (w, h),
            (Some(w), None) => (w, (w as f64 / original_aspect_ratio).round() as usize),
            (None, Some(h)) => ((h as f64 * original_aspect_ratio).round() as usize, h),
            (None, None) => (self.width, self.height),
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

    fn warn_if_resolution_adjusted(&self, arguments: &Arguments, final_width: usize, final_height: usize) {
        let requested_width = arguments.width.unwrap_or(0);
        let requested_height = arguments.height.unwrap_or(0);

        if (requested_width > 0 && self.width != requested_width) || 
           (requested_height > 0 && self.height != requested_height) {
            println!(
                "Warning: Resolution adjusted from {}x{} to {}x{} to maintain aspect ratio and scaling factor.",
                requested_width, requested_height, final_width, final_height
            );
        }
    }

}