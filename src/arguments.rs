use anyhow::{anyhow, bail, Context, Result};
use std::path::Path;
use std::process::{exit, Command};

pub struct Arguments {
    input: String,
    output: Option<String>,
    formats: Vec<String>,
    pub files: Vec<(String, String)>,
    pub width: Option<usize>,
    pub height: Option<usize>,
    pub scale: Option<f32>,
    pub encoder: String,
    pub model: String,
    pub replace_output: bool,
}

impl Default for Arguments {
    fn default() -> Self {
        let formats = vec!["mp4", "mov", "mkv", "webm", "avi", "flv"]
            .into_iter()
            .map(|s| s.to_owned())
            .collect::<Vec<String>>();

        let input = String::new();
        let output = None;
        let width = None;
        let height = None;
        let scale = Some(2.0);
        let encoder = String::from("libx264");
        let files = Vec::new();
        let model = String::from("realesrgan");
        let replace_output = false;
        
        Self {
            input,
            output,
            width,
            height,
            scale,
            encoder,
            files,
            formats,
            model,
            replace_output,
        }
    }
}

impl Arguments {
    pub fn parse() -> Result<Self> {
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

    fn get_next_arg(&self, args: &[String], index: &mut usize, arg_name: &str) -> Result<String> {
        *index += 1;
        args.get(*index)
            .cloned()
            .ok_or_else(|| anyhow!("Missing value for argument: {arg_name}"))
    }
    
    fn parse_arguments(&mut self) -> Result<()> {
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
                "-s" | "--scale" => self.scale = Some(self.parse_numeric_arg(&args, &mut i, "scale")?),
                "-e" | "--encoder" => self.encoder = self.get_next_arg(&args, &mut i, "encoder")?,
                "-m" | "--model" => self.model = self.get_next_arg(&args, &mut i, "model")?,
                "--replace_output" => self.replace_output = true,
                "--help" => Self::print_help(),
                argument => bail!("Invalid argument: {argument}"),
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
        println!("  -s, --scale FACTOR         Set the upscaling factor (1.5, 2.0, 4.0, etc.) (default: 2.0)");
        println!("  -e, --encoder ENCODER      Choose the video encoder (default: libx264)");
        println!("  -m, --model MODEL          Select the AI model for upscaling: (default: realesrgan)");
        println!("                             realcugan | realesrgan | realesrgan-anime | realesr-anime");
        println!("      --duplicate_threshold  Set the similarity threshold for identifying duplicate frames (default: 1.0)");
        println!("      --replace_output       Replace the output file if it already exists");
        println!("      --help                 Display this help message and exit");
        exit(0);
    }

    fn parse_numeric_arg<O: std::str::FromStr>(&self, args: &[String], index: &mut usize, argument_name: &str) -> Result<O> {
        self.get_next_arg(args, index, argument_name)?
            .parse()
            .map_err(|_| anyhow!("Argument '{argument_name}' must be a number"))
    }

    fn set_input_files(&mut self) -> Result<()> {
        if self.input.is_empty() {
            bail!("Input is empty");
        }
        
        let path = Path::new(&self.input);
        if !path.exists() {
            bail!("Input file or directory not found: {path_display}", path_display = path.display());
        }

        let input_files = if path.is_dir() {
            self.get_files_from_directory(path)?
        } else {
            vec![self.get_file_if_valid(path)
                .ok_or_else(|| anyhow!("Input file not found: {path_display}", path_display = path.display()))?]
        };

        if input_files.is_empty() {
            bail!("No valid input files found");
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

    fn get_files_from_directory(&self, dir: &Path) -> Result<Vec<String>> {
        let files = std::fs::read_dir(dir)
            .with_context(|| format!("Failed to read directory: {dir_display}", dir_display = dir.display()))?
            .filter_map(|entry| entry.ok().and_then(|e| self.get_file_if_valid(&e.path())))
            .collect::<Vec<_>>();

        Ok(files)
    }

    fn set_output_files(&mut self) -> Result<()> {
        if let Some(output) = self.output.take() {
            self.set_output_with_path(&output)?;
        } else {
            self.set_default_output()?;
        }

        if self.files.iter().any(|(_, output)| output.is_empty()) {
            bail!("Failed to create output file: {input}", input = self.input);
        }

        self.files = self.files
            .clone()
            .into_iter()
            .filter(|(_, output)| {
                if Path::new(output).exists() && !self.replace_output {
                    println!("Skipping {output} output file already exists");
                    false
                } else {
                    true
                }
            })
            .collect();
    
        Ok(())
    }

    fn set_output_with_path(&mut self, output: &str) -> Result<()> {
        let path = Path::new(output);
        if (path.exists() && path.is_file()) || path.extension().is_some() {
            if self.files.len() > 1 {
                bail!("Output file already exists: {path_display}", path_display = path.display());
            }
            self.set_single_output_file(path)?;
        } else {
            self.set_multiple_output_files(path)?;
        }

        Ok(())
    }

    fn set_single_output_file(&mut self, output_path: &Path) -> Result<()> {
        if let Some(output_dir) = output_path.parent() {
            std::fs::create_dir_all(output_dir)
                .with_context(|| format!("Failed to create output directory: {output_dir_display}", output_dir_display = output_dir.display()))?;
            self.files[0].1 = output_path.to_string_lossy().into_owned();
            Ok(())
        } else {
            bail!("Failed to create output file: {output_path_display}", output_path_display = output_path.display())
        }
    }

    fn set_multiple_output_files(&mut self, output_path: &Path) -> Result<()> {
        for (input, output) in &mut self.files {
            let input_path = Path::new(input);
            let input_file_name = input_path.file_name().ok_or_else(|| anyhow!("Invalid input file name: {input}", input = input))?;
            *output = output_path.join(input_file_name).to_string_lossy().into_owned();
        }
        std::fs::create_dir_all(output_path)
            .with_context(|| format!("Failed to create output directory: {output_path_display}", output_path_display = output_path.display()))?;
        Ok(())
    }

    fn set_default_output(&mut self) -> Result<()> {
        for (input, output) in &mut self.files {
            let input_path = Path::new(input);
            let output_path = input_path.parent().unwrap_or_else(|| Path::new("."));
            let file_stem = input_path.file_stem().ok_or_else(|| anyhow!("Invalid input file name: {input}", input = input))?;
            let mut file_name = file_stem.to_string_lossy().to_string();
            file_name.push_str("_converted");
            file_name.push_str(&input_path.extension().unwrap_or_default().to_string_lossy());
            *output = output_path.join(file_name).to_string_lossy().into_owned();
        }
        Ok(())
    }

    fn check_ffmpeg(&self) -> Result<()> {
        match Command::new("ffmpeg").spawn() {
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                bail!("Cannot find ffmpeg: {error}", error = e)
            },
            Err(_) => Ok(()),
            Ok(mut c) => {
                let _ = c.kill();
                Ok(())
            },
        }
    }

    fn validate_model(&self) -> Result<()> {
        match self.model.as_str() {
            "realcugan" | "realesrgan" | "realesrgan-anime" | "realesr-anime" => Ok(()),
            model => bail!("Invalid model: {model}. Must be realcugan, realesrgan, realesrgan-anime or realesr-anime"),
        }
    }

    fn validate_resolution_and_scale(&mut self) -> Result<()> {
        if self.scale.is_some() && (self.width.is_some() || self.height.is_some()) {
            bail!("Cannot specify both scale factor and explicit width/height");
        }

        if let Some(width) = self.width 
            && !(16..=7680).contains(&width) {
            bail!("Invalid width: {width}. Must be between 16 and 7680");
        }
    
        if let Some(height) = self.height 
            && !(16..=4320).contains(&height) {
            bail!("Invalid height: {height}. Must be between 16 and 4320");
        }

        if let Some(scale) = self.scale {
            match self.model.as_str() {
                "realcugan" | "realesr-anime" => {
                    if scale > 4.0 {
                        bail!("Scale factor {scale} not supported for model {model}. Supported scales: 1, 2, 3, 4", scale = scale, model = self.model);
                    }
                }
                "realesrgan" | "realesrgan-anime" => {
                    if scale > 4.0 {
                        bail!("Scale factor {scale} not supported for model {model}. Only scale 4 is supported", scale = scale, model = self.model);
                    }
                }
                _ => {}
            }
        }
    
        Ok(())
    }

    fn validate_encoder(&self) -> Result<()> {
        let output = Command::new("ffmpeg")
            .args(["-hide_banner", "-encoders"])
            .output()
            .with_context(|| "Failed to execute ffmpeg")?;

        let encoders = String::from_utf8_lossy(&output.stdout);
        
        if !encoders.contains(&self.encoder) {
            bail!("Invalid encoder: {encoder}. Available encoders: {encoders}", encoder = self.encoder, encoders = encoders);
        }

        Ok(())
    }
}