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
    pub model: String,
    pub duplicate_threshold: f64,
    pub replace_output: bool
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
            duplicate_threshold: 1.0,
            replace_output: false
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

    fn get_next_arg(&self, args: &[String], index: &mut usize, arg_name: &str) -> Result<String, Error> {
        *index += 1;
        args.get(*index).cloned().ok_or_else(|| Error::new(format!("Missing value for argument: {}", arg_name)))
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
                "--replace_output" => self.replace_output = true,
                "--duplicate_threshold" => self.duplicate_threshold = self.parse_numeric_arg(&args, &mut i, "duplicate_threshold")?,
                "--help" => Self::print_help(),
                _ => return Err(Error::new(format!("Invalid argument: {}", args[i]))),
            }
            i += 1;
        }
    
        Ok(())
    }

    fn print_help() {
        println!("Usage: program_name [OPTIONS]");
        println!();
        println!("Options:");
        println!("  -i, --input FILE           Specify the input video file or directory");
        println!("  -o, --output FILE          Specify the output video file");
        println!("  -w, --width WIDTH          Set the target video width (in pixels)");
        println!("  -h, --height HEIGHT        Set the target video height (in pixels)");
        println!("  -e, --encoder ENCODER      Choose the video encoder (default: libx264)");
        println!("  -m, --model MODEL          Select the AI model for upscaling: (default: realesrgan)");
        println!("                             realcugan | realesrgan | realesrgan-anime | realesr-anime");
        println!("      --duplicate_threshold  Set the similarity threshold for identifying duplicate frames (default: 1.0)");
        println!("      --replace_output       Replace the output file if it already exists");
        println!("      --help                 Display this help message and exit");
        exit(0);
    }

    fn parse_numeric_arg<O: std::str::FromStr>(&self, args: &[String], index: &mut usize, arg_name: &str) -> Result<O, Error> {
        let value = self.get_next_arg(args, index, arg_name)?;
        value.parse().map_err(|_| Error::new(format!("Argument '{}' must be a number", arg_name)))
    }

    fn set_input_files(&mut self) -> Result<(), Error> {
        if self.input.is_empty() {
            return Err(Error::new("Input is empty".to_string()));
        }
        
        let path = Path::new(&self.input);
        if !path.exists() {
            return Err(Error::new(format!("Input file or directory not found: {}", path.display())));
        }

        let input_files = if path.is_dir() {
            self.get_files_from_directory(path)?
        } else {
            vec![self.get_file_if_valid(path).ok_or_else(|| Error::new(format!("Input file not found: {}", path.display())))?]
        };

        if input_files.is_empty() {
            return Err(Error::new("No valid input files found".to_string()));
        }

        self.files = input_files.into_iter().map(|f| (f, String::new())).collect();
        Ok(())
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

    fn get_files_from_directory(&self, dir: &Path) -> Result<Vec<String>, Error> {
        std::fs::read_dir(dir)
            .map_err(|e| Error::new(format!("Failed to read directory: {}", e)))?
            .filter_map(|entry| entry.ok().and_then(|e| self.get_file_if_valid(&e.path())))
            .collect::<Vec<_>>()
            .into_iter()
            .map(Ok)
            .collect()
    }

    fn set_output_files(&mut self) -> Result<(), Error> {
        if let Some(output) = self.output.take() {
            self.set_output_with_path(&output)?;
        } else {
            self.set_default_output()?;
        }

        if self.files.iter().any(|(_, output)| output.is_empty()) {
            return Err(Error::new(format!("Failed to create output file: {}", self.input)));
        }

        self.files = self.files
            .clone()
            .into_iter()
            .filter(|(_, output)| {
                if Path::new(output).exists() && !self.replace_output {
                    println!("Skipping {} output file already exists", output);
                    false
                } else {
                    true
                }
            })
            .collect();
    
        Ok(())
    }

    fn set_output_with_path(&mut self, output: &str) -> Result<(), Error> {
        let path = Path::new(output);
        if (path.exists() && path.is_file()) || path.extension().is_some() {
            if self.files.len() > 1 {
                return Err(Error::new(format!("Output file already exists: {}", path.display())));
            }
            self.set_single_output_file(path)?;
        } else {
            self.set_multiple_output_files(path)?;
        }

        Ok(())
    }

    fn set_single_output_file(&mut self, output_path: &Path) -> Result<(), Error> {
        if let Some(output_dir) = output_path.parent() {
            std::fs::create_dir_all(output_dir)
                .map_err(|e| Error::new(format!("Failed to create output directory: {}", e)))?;
            self.files[0].1 = output_path.to_string_lossy().into_owned();
        } else {
            return Err(Error::new(format!("Failed to create output file: {}", output_path.display())));
        }
        Ok(())
    }

    fn set_multiple_output_files(&mut self, output_path: &Path) -> Result<(), Error> {
        for (input, output) in &mut self.files {
            let input_path = Path::new(input);
            *output = output_path.join(input_path.file_name().unwrap()).to_string_lossy().into_owned();
        }
        std::fs::create_dir_all(output_path)
            .map_err(|e| Error::new(format!("Failed to create output directory: {}", e)))?;
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
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Err(Error::new(format!("Cannot find ffmpeg: {}", &e))),
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
            _ => Err(Error::new(format!("Invalid model: {}. Must be realcugan, realesrgan, realesrgan-anime or realesr-anime", self.model))),
        }
    }

    fn validate_resolution_and_scale(&mut self) -> Result<(), Error> {
        if let Some(width) = self.width {
            if width < 16 || width > 7680 {
                return Err(Error::new(format!("Invalid width: {}. Must be between 16 and 7680", width)));
            }
        }
    
        if let Some(height) = self.height {
            if height < 16 || height > 4320 {
                return Err(Error::new(format!("Invalid height: {}. Must be between 16 and 4320", height)));
            }
        }
    
        Ok(())
    }

    fn validate_encoder(&self) -> Result<(), Error> {
        let output = Command::new("ffmpeg")
            .args(&["-hide_banner", "-encoders"])
            .output()
            .map_err(|e| Error::new(format!("Failed to execute ffmpeg: {}", e)))?;

        let encoders = String::from_utf8_lossy(&output.stdout);
        
        if !encoders.contains(&self.encoder) {
            return Err(Error::new(format!("Invalid encoder: {}. Available encoders: {}", self.encoder, encoders)));
        }

        Ok(())
    }
}