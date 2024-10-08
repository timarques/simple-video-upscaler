use crate::error::Error;

use realcugan_rs::RealCugan;

use std::path::Path;
use std::env;
use std::process::Command;

pub struct Args {
    target_width: Option<usize>,
    target_height: Option<usize>,
    scale: usize,
    pub input: String,
    pub output: String,
    pub frame_rate: f64,
    pub frame_total: usize,
    pub width: usize,
    pub height: usize,
    pub model: RealCugan,
    pub encoder: String,
}

impl Args {
    pub fn parse() -> Result<Self, Error> {

        let mut args = Self {
            input: String::new(),
            output: String::new(),
            target_width: None,
            target_height: None,
            frame_rate: 0.0,
            frame_total: 0,
            width: 0,
            height: 0,
            scale: 2,
            model: RealCugan::from_model(realcugan_rs::Model::Se2xConservative),
            encoder: String::from("libx264"),
        };

        args.parse_args()?;
        args.validate_ffmpeg_binary()?;
        args.validate_encoder()?;
        args.validate_paths()?;
        args.set_frame_rate_total_and_resolution()?;
        args.set_scale_and_resolution();
        args.set_model();

        Ok(args)
    }

    fn print_help() {
        println!("Usage: program_name [OPTIONS]");
        println!("Options:");
        println!("  -i, --input FILE    Input video file");
        println!("  -o, --output FILE   Output video file");
        println!("  --width WIDTH       Target width");
        println!("  --height HEIGHT     Target height");
        println!("  --encoder ENCODER   Video encoder (default: libx264)");
        println!("  -h, --help          Show this help message");
    }

    pub fn print_options(&self) {
        println!("Input:      {}", self.input);
        println!("Output:     {}", self.output);
        println!("Scale:      {}x", self.scale);
        println!("Encoder:    {}", self.encoder);
        println!("Resolution: {}x{}", self.width, self.height);
    }

    fn parse_args(&mut self) -> Result<(), Error> {
        let args: Vec<String> = env::args().collect();

        if args.len() == 1 ||
            args.contains(&"-h".to_string()) ||
            args.contains(&"--help".to_string())
        {
            Self::print_help();
            std::process::exit(0);
        }
        
        let mut input = None;
        let mut output = None;
        let mut target_width = None;
        let mut target_height = None;
        let mut encoder = String::from("libx264");

        let mut i = 1;
        while i < args.len() {
            match args[i].as_str() {
                "-i" | "--input" => {
                    i += 1;
                    if let Some(arg) = args.get(i) {
                        input = Some(arg.to_owned());
                    } else {
                        return Err(Error::EmptyArgument("input".to_string()));
                    }
                }
                "-o" | "--output" => {
                    i += 1;
                    if let Some(arg) = args.get(i) {
                        output = Some(arg.to_owned());
                    } else {
                        return Err(Error::EmptyArgument("output".to_string()));
                    }
                }
                "--width" => {
                    i += 1;
                    if let Some(arg) = args.get(i) {
                        target_width = Some(arg.parse().map_err(|_| Error::InvalidArgument("width".to_string()))?);
                    } else {
                        return Err(Error::EmptyArgument("width".to_string()));
                    }
                }
                "--height" => {
                    i += 1;
                    if let Some(arg) = args.get(i) {
                        target_height = Some(arg.parse().map_err(|_| Error::InvalidArgument("height".to_string()))?);
                    } else {
                        return Err(Error::EmptyArgument("height".to_string()));
                    }
                }
                "--encoder" => {
                    i += 1;
                    if let Some(arg) = args.get(i) {
                        encoder = arg.to_owned();
                    } else {
                        return Err(Error::EmptyArgument("encoder".to_string()));
                    }
                }
                _ => return Err(Error::UnknownArgument(args[i].clone())),
            }
            i += 1;
        }

        let input = input.ok_or_else(|| Error::MissingArgument("input".to_string()))?;
        let output = output.ok_or_else(|| Error::MissingArgument("output".to_string()))?;
        self.input = input;
        self.output = output;
        self.target_width = target_width;
        self.target_height = target_height;
        self.encoder = encoder;
        Ok(())
    }

    fn validate_ffmpeg_binary(&self) -> Result<(), Error> {
        Command::new("ffmpeg")
            .arg("-version")
            .output()
            .map(|_| ())
            .map_err(|_| Error::FFmpegNotAvailable)
    }

    fn validate_encoder(&self) -> Result<(), Error> {
        let output = Command::new("ffmpeg")
            .args(&["-encoders"])
            .output()
            .map_err(|_| Error::FfmpegFailed)?;

        let encoders = String::from_utf8_lossy(&output.stdout);
        
        if !encoders.contains(&self.encoder) {
            return Err(Error::UnsupportedEncoder(self.encoder.clone()));
        }

        Ok(())
    }

    fn validate_paths(&self) -> Result<(), Error> {
        let input_path = Path::new(&self.input);
        if !input_path.exists() || !input_path.is_file() {
            return Err(Error::InvalidInputPath);
        }
    
        let output_path = Path::new(&self.output);
        if let Some(output_dir) = output_path.parent() {
            if std::fs::metadata(output_dir)?.permissions().readonly() {
                return Err(Error::InvalidOutputPath);
            }
            std::fs::create_dir_all(output_dir)?;
        } else {
            return Err(Error::InvalidOutputPath);
        }
    
        Ok(())
    }

    fn set_frame_rate_total_and_resolution(&mut self) -> Result<(), Error> {
            let output = Command::new("ffprobe")
            .args(&[
                "-v", "error",
                "-select_streams", "v:0",
                "-count_frames",
                "-show_entries", "stream=nb_read_frames,r_frame_rate,width,height",
                "-of", "default=noprint_wrappers=1",
                &self.input,
            ])
            .output()?;
    
        let data = String::from_utf8_lossy(&output.stdout);
    
        for line in data.lines() {
            if line.starts_with("nb_read_frames=") {
                self.frame_total = line.split('=').nth(1).and_then(|s| s.parse().ok()).unwrap_or(0);
            } else if line.starts_with("r_frame_rate=") {
                let fps_parts: Vec<&str> = line.split('=').nth(1).unwrap_or("").split('/').collect();
                if fps_parts.len() == 2 {
                    let num = fps_parts[0].parse::<f64>().unwrap_or(0.0);
                    let den = fps_parts[1].parse::<f64>().unwrap_or(1.0);
                    self.frame_rate = num / den;
                }
            } else if line.starts_with("width=") {
                self.width = line.split('=').nth(1).and_then(|s| s.parse().ok()).unwrap_or(0);
            } else if line.starts_with("height=") {
                self.height = line.split('=').nth(1).and_then(|s| s.parse().ok()).unwrap_or(0);
            }
        }
        Ok(())
    }

    fn determine_target_dimensions(&self, original_aspect_ratio: f64) -> (usize, usize) {
        match (self.target_width, self.target_height) {
            (Some(w), Some(h)) => (w, h),
            (Some(w), None) => (w, (w as f64 / original_aspect_ratio).round() as usize),
            (None, Some(h)) => ((h as f64 * original_aspect_ratio).round() as usize, h),
            (None, None) => (self.width, self.height),
        }
    }

    fn adjust_dimensions_for_aspect_ratio(&self, width: usize, height: usize, original_aspect_ratio: f64) -> (usize, usize) {
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

    fn calculate_resolution(&mut self) -> (usize, usize) {
        let original_aspect_ratio = self.width as f64 / self.height as f64;

        let (target_width, target_height) = self.determine_target_dimensions(original_aspect_ratio);
        let (adjusted_width, adjusted_height) = self.adjust_dimensions_for_aspect_ratio(target_width, target_height, original_aspect_ratio);

        (adjusted_width, adjusted_height)
    }

    fn set_scale_and_resolution(&mut self) {
        let (width, height) = self.calculate_resolution();
        self.scale = (2..=4).rev()
            .find(|&scale| {
                width >= self.width * scale ||
                height >= self.height * scale
            })
            .unwrap_or(2);

        self.width = std::cmp::min(width, width * self.scale);
        self.height = std::cmp::min(height, height * self.scale);

        if (self.target_width.is_some() && Some(self.width) != self.target_width) || 
            (self.target_height.is_some() && Some(self.height) != self.target_height) {
            println!(
                "Warning: Resolution adjusted from {}x{} to {}x{} to maintain aspect ratio and scaling factor.",
                self.target_width.unwrap(), self.target_height.unwrap(), self.width, self.height
            );
        }
    }

    fn set_model(&mut self) {
        let scale_factor = self.scale;
        match scale_factor {
            2 => self.model = RealCugan::from_model(realcugan_rs::Model::Se2xConservative),
            3 => self.model = RealCugan::from_model(realcugan_rs::Model::Se3xConservative),
            4 => self.model = RealCugan::from_model(realcugan_rs::Model::Se4xConservative),
            _ => unimplemented!()
        }
    }

}