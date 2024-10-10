use std::error::Error as StdError;
use std::fmt;
use image::error::ImageError;

#[derive(Debug)]
pub enum Error {
    ImageError(ImageError),
    Io(std::io::Error),
    InvalidImageBuffer,
    FrameBufferOverflow,
    UpscaleError(String),
    FfmpegFailed,
    FFmpegNotAvailable,
    SendError,
    InputFilesNotFound,
    OutputFilesNotFound,
    InvalidInputPath,
    InvalidOutputPath,
    EmptyArgument(String),
    InvalidArgument(String),
    MissingArgument(String),
    UnsupportedEncoder(String),
    MixedArguments(String, String),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::ImageError(e) => write!(f, "Failed to process image: {}", e),
            Error::Io(e) => write!(f, "I/O operation failed: {}", e),
            Error::InvalidImageBuffer => write!(f, "Invalid image buffer detected"),
            Error::FrameBufferOverflow => write!(f, "Frame buffer overflow encountered"),
            Error::UpscaleError(e) => write!(f, "Upscaling operation failed: {}", e),
            Error::FfmpegFailed => write!(f, "FFmpeg command execution failed"),
            Error::FFmpegNotAvailable => write!(f, "FFmpeg is not available on this system"),
            Error::SendError => write!(f, "Error sending data across channels"),
            Error::InputFilesNotFound => write!(f, "Input files not found"),
            Error::OutputFilesNotFound => write!(f, "Output files not found"),
            Error::InvalidInputPath => write!(f, "The specified input path is invalid"),
            Error::InvalidOutputPath => write!(f, "The specified output path is invalid"),
            Error::EmptyArgument(arg) => write!(f, "Argument cannot be empty: {}", arg),
            Error::MixedArguments(arg, arg2) => write!(f, "Arguments cannot be mixed {} and {}", arg, arg2),
            Error::InvalidArgument(arg) => write!(f, "Invalid argument provided: {}", arg),
            Error::MissingArgument(arg) => write!(f, "Required argument is missing: {}", arg),
            Error::UnsupportedEncoder(encoder) => write!(f, "The encoder is not supported: {}", encoder),
        }
    }
}

impl StdError for Error {
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        match self {
            Error::ImageError(e) => Some(e),
            Error::Io(e) => Some(e),
            _ => None,
        }
    }
}

impl From<ImageError> for Error {
    fn from(err: ImageError) -> Self {
        Error::ImageError(err)
    }
}

impl From<std::io::Error> for Error {
    fn from(err: std::io::Error) -> Self {
        Error::Io(err)
    }
}