use crate::error::Error;

use realcugan_rs::RealCugan;

use std::path::Path;
use std::env;
use std::process::Command;

struct VideoInfo {
    width: usize,
    height: usize,
    frame_rate: f64,
    frame_count: usize,
}

pub struct Args {
    target_width: Option<usize>,
    target_height: Option<usize>,
    scale: usize,
    pub input: String,
    pub output: String,
    pub frame_rate: f64,
    pub frame_count: usize,
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
            frame_count: 0,
            width: 0,
            height: 0,
            scale: 2,
            model: RealCugan::from_model(realcugan_rs::Model::Se2xConservative),
            encoder: String::from("libx264"),
        };

        args.parse_args()?;
        args.validate_paths()?;
        args.validate_ffmpeg_binary()?;
        
        let video_info_handle = args.get_video_info();
        let validate_encoder_handle = args.validate_encoder();

        validate_encoder_handle.join().unwrap()?;
        let video_info = video_info_handle.join().unwrap()?;
        args.frame_rate = video_info.frame_rate;
        args.frame_count = video_info.frame_count;
        args.width = video_info.width;
        args.height = video_info.height;

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
        self.input = Path::new(&input).display().to_string();
        self.output = Path::new(&output).display().to_string();
        self.target_width = target_width;
        self.target_height = target_height;
        self.encoder = encoder;
        Ok(())
    }

    fn validate_ffmpeg_binary(&self) -> Result<(), Error> {
        match Command::new("ffmpeg").spawn() {
            Err(e) => {
                if e.kind() == std::io::ErrorKind::NotFound {
                    Err(Error::FFmpegNotAvailable)
                } else {
                    Ok(())
                }
            },
            Ok(mut c) => {
                let _ = c.kill();
                Ok(())
            },
        }
    }

    fn validate_encoder(&self) -> std::thread::JoinHandle<Result<(), Error>> {
        let encoder = self.encoder.clone();
        std::thread::spawn(move || -> Result<(), Error> {
            let output = Command::new("ffmpeg")
                .args(&["-encoders"])
                .output()
                .map_err(|_| Error::FfmpegFailed)?;

            let encoders = String::from_utf8_lossy(&output.stdout);
            
            if !encoders.contains(&encoder) {
                return Err(Error::UnsupportedEncoder(encoder));
            }

            Ok(())
        })
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

    fn get_video_info(&self) -> std::thread::JoinHandle<Result<VideoInfo, Error>> {
        let input_file = self.input.clone();
        std::thread::spawn(move || {
            let output = Command::new("ffprobe")
                .args(&[
                    "-v", "error",
                    "-select_streams", "v:0",
                    "-count_frames",
                    "-show_entries", "stream=nb_read_frames,r_frame_rate,width,height",
                    "-of", "default=noprint_wrappers=1",
                    &input_file,
                ])
                .output()?;

            let mut video_info = VideoInfo {
                width: 0,
                height: 0,
                frame_count: 0,
                frame_rate: 0.0
            };

            let data = String::from_utf8_lossy(&output.stdout);
        
            for line in data.lines() {
                if line.starts_with("nb_read_frames=") {
                    video_info.frame_count = line.split('=').nth(1).and_then(|s| s.parse().ok()).unwrap_or(0);
                } else if line.starts_with("r_frame_rate=") {
                    let fps_parts: Vec<&str> = line.split('=').nth(1).unwrap_or("").split('/').collect();
                    if fps_parts.len() == 2 {
                        let num = fps_parts[0].parse::<f64>().unwrap_or(0.0);
                        let den = fps_parts[1].parse::<f64>().unwrap_or(1.0);
                        video_info.frame_rate = num / den;
                    }
                } else if line.starts_with("width=") {
                    video_info.width = line.split('=').nth(1).and_then(|s| s.parse().ok()).unwrap_or(0);
                } else if line.starts_with("height=") {
                    video_info.height = line.split('=').nth(1).and_then(|s| s.parse().ok()).unwrap_or(0);
                }
            }

            Ok(video_info)
        })
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