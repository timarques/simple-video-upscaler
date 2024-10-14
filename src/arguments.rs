use crate::error::Error;

use std::path::Path;
use std::process::{exit, Command};

pub struct Arguments {
    input: String,
    output: Option<String>,
    formats: Vec<String>,
    pub files: Vec<(String, String)>,
    pub width: Option<usize>,
    pub height: Option<usize>,
    pub encoder: String,
    pub model: String
}

impl Default for Arguments {
    fn default() -> Self {
        let formats = vec!["mp4", "mov", "mkv", "webm", "avi", "flv"]
            .into_iter()
            .map(|s| s.to_owned())
            .collect::<Vec<String>>();
        
        Self {
            input: String::new(),
            output: None,
            width: None,
            height: None,
            encoder: String::from("libx264"),
            files: Vec::new(),
            formats,
            model: String::from("realesrgan"),
        }
    }
}

impl Arguments {
    pub fn parse() -> Result<Self, Error> {
        let mut arguments = Self::default();

        arguments.check_ffmpeg()?;
        arguments.parse_arguments()?;
        arguments.validate_encoder()?;
        arguments.validate_model()?;
        arguments.validate_resolution_and_scale()?;
        arguments.set_input_files()?;
        arguments.set_output_files()?;

        Ok(arguments)
    }

    fn parse_arguments(&mut self) -> Result<(), Error> {
        let args: Vec<String> = std::env::args().collect();

        if args.len() < 2 {
            Self::print_help();
        }
        
        let mut i = 1;
        while i < args.len() {
            match args[i].as_str() {
                "-i" | "--input" => self.input = self.get_next_arg(&args, &mut i, "input")?,
                "-o" | "--output" => self.output = Some(self.get_next_arg(&args, &mut i, "output")?),
                "-w" | "--width" => self.width = Some(self.parse_numeric_arg(&args, &mut i, "width")?),
                "-h" | "--height" => self.height = Some(self.parse_numeric_arg(&args, &mut i, "height")?),
                "-e" | "--encoder" => self.encoder = self.get_next_arg(&args, &mut i, "encoder")?,
                "-m" | "--model" => self.model = self.get_next_arg(&args, &mut i, "model")?,
                "--help" => Self::print_help(),
                _ => return Err(Error::InvalidArgument(args[i].to_string())),
            }
            i += 1;
        }

        Ok(())
    }

    fn get_next_arg(&self, args: &[String], index: &mut usize, arg_name: &str) -> Result<String, Error> {
        *index += 1;
        args.get(*index).cloned().ok_or_else(|| Error::EmptyArgument(arg_name.to_string()))
    }

    fn parse_numeric_arg(&self, args: &[String], index: &mut usize, arg_name: &str) -> Result<usize, Error> {
        let value = self.get_next_arg(args, index, arg_name)?;
        value.parse().map_err(|_| Error::InvalidArgument(arg_name.to_string()))
    }

    fn print_help() {
        println!("Usage: program_name [OPTIONS]");
        println!();
        println!("Options:");
        println!("  -i, --input FILE         Specify the input video file or directory");
        println!("  -o, --output FILE        Specify the output video file");
        println!("  -w, --width WIDTH        Set the target video width (in pixels)");
        println!("  -h, --height HEIGHT      Set the target video height (in pixels)");
        println!("  -e, --encoder ENCODER    Choose the video encoder (default: libx264)");
        println!("  -m, --model MODEL        Select the AI model for upscaling: (default: realesrgan)");
        println!("                           realcugan | realesrgan | realesrgan-anime | realesr-anime");
        println!("      --help               Display this help message and exit");
        exit(0);
    }

    fn set_input_files(&mut self) -> Result<(), Error> {
        if self.input.is_empty() {
            return Err(Error::InvalidInputPath);
        }
        
        let path = Path::new(&self.input);
        if !path.exists() {
            return Err(Error::InvalidInputPath);
        }

        let input_files = if path.is_dir() {
            self.get_files_from_directory(path)?
        } else {
            vec![self.get_file_if_valid(path).ok_or(Error::InvalidInputPath)?]
        };

        if input_files.is_empty() {
            return Err(Error::InputFilesNotFound);
        }

        self.files = input_files.into_iter().map(|f| (f, String::new())).collect();
        Ok(())
    }

    fn get_files_from_directory(&self, dir: &Path) -> Result<Vec<String>, Error> {
        let files = std::fs::read_dir(dir)?
            .filter_map(|entry| entry.ok().and_then(|e| self.get_file_if_valid(&e.path())))
            .collect();
        Ok(files)
    }

    fn get_file_if_valid(&self, path: &Path) -> Option<String> {
        path.is_file().then(|| {
            path.extension()
                .and_then(std::ffi::OsStr::to_str)
                .map(str::to_lowercase)
                .filter(|ext| self.formats.contains(ext))
                .map(|_| path.to_string_lossy().into_owned())
        }).flatten()
    }

    fn set_output_files(&mut self) -> Result<(), Error> {
        if let Some(output) = self.output.take() {
            self.set_output_with_path(&output)?;
        } else {
            self.set_default_output()?;
        }

        if self.files.iter().all(|(_, output)| output.is_empty()) {
            return Err(Error::OutputFilesNotFound);
        }
    
        Ok(())
    }

    fn set_output_with_path(&mut self, output: &str) -> Result<(), Error> {
        let path = Path::new(output);
        if (path.exists() && path.is_file()) || path.extension().is_some() {
            if self.files.len() > 1 {
                return Err(Error::InvalidOutputPath);
            }
            self.set_single_output_file(path)?;
        } else {
            self.set_multiple_output_files(path)?;
        }

        Ok(())
    }

    fn set_single_output_file(&mut self, output_path: &Path) -> Result<(), Error> {
        if let Some(output_dir) = output_path.parent() {
            std::fs::create_dir_all(output_dir)?;
            self.files[0].1 = output_path.to_string_lossy().into_owned();
        } else {
            return Err(Error::InvalidOutputPath);
        }
        Ok(())
    }

    fn set_multiple_output_files(&mut self, output_path: &Path) -> Result<(), Error> {
        for (input, output) in &mut self.files {
            let input_path = Path::new(input);
            *output = output_path.join(input_path.file_name().unwrap()).to_string_lossy().into_owned();
        }
        std::fs::create_dir_all(output_path)?;
        Ok(())
    }

    fn set_default_output(&mut self) -> Result<(), Error> {
        for (input, output) in &mut self.files {
            let input_path = Path::new(input);
            let output_path = input_path.parent().unwrap_or_else(|| Path::new("."));
            let mut file_name = input_path.file_stem().unwrap().to_string_lossy().to_string();
            file_name.push_str("_converted");
            file_name.push_str(&input_path.extension().unwrap().to_string_lossy());
            *output = output_path.join(file_name).to_string_lossy().into_owned();
        }
        Ok(())
    }

    fn check_ffmpeg(&self) -> Result<(), Error> {
        match Command::new("ffmpeg").spawn() {
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Err(Error::FFmpegNotAvailable),
            Err(_) => Ok(()),
            Ok(mut c) => {
                let _ = c.kill();
                Ok(())
            },
        }
    }

    fn validate_model(&self) -> Result<(), Error> {
        match self.model.as_str() {
            "realcugan" | "realesrgan" | "realesrgan-anime" | "realesr-anime" => Ok(()),
            _ => Err(Error::InvalidArgument(format!("model must be realcugan, realesrgan, realesrgan-anime or realesr-anime, got {}", self.model))),
        }
    }

    fn validate_resolution_and_scale(&mut self) -> Result<(), Error> {
        if let Some(width) = self.width {
            if width < 16 || width > 7680 {
                return Err(Error::InvalidArgument(format!("width must be between 16 and 7680, got {}", width)));
            }
        }
    
        if let Some(height) = self.height {
            if height < 16 || height > 4320 {
                return Err(Error::InvalidArgument(format!("height must be between 16 and 4320, got {}", height)));
            }
        }
    
        Ok(())
    }

    fn validate_encoder(&self) -> Result<(), Error> {
        let output = Command::new("ffmpeg")
            .args(&["-hide_banner", "-encoders"])
            .output()
            .map_err(|_| Error::FfmpegFailed)?;

        let encoders = String::from_utf8_lossy(&output.stdout);
        
        if !encoders.contains(&self.encoder) {
            return Err(Error::UnsupportedEncoder(self.encoder.clone()));
        }

        Ok(())
    }
}