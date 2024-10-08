use thiserror::Error;
use image::error::ImageError;

#[derive(Error, Debug)]
pub enum Error {
    #[error("Failed to process image: {0}")]
    ImageError(#[from] ImageError),
    #[error("I/O operation failed: {0}")]
    Io(#[from] std::io::Error),
    #[error("Invalid image buffer detected")]
    InvalidImageBuffer,
    #[error("Frame buffer overflow encountered")]
    FrameBufferOverflow,
    #[error("Upscaling operation failed: {0}")]
    UpscaleError(String),

    #[error("FFmpeg command execution failed")]
    FfmpegFailed,
    #[error("FFmpeg is not available on this system")]
    FFmpegNotAvailable,

    #[error("Error sending data across channels")]
    SendError,

    #[error("The specified input path is invalid")]
    InvalidInputPath,
    #[error("The specified output path is invalid")]
    InvalidOutputPath,
    #[error("Argument cannot be empty: {0}")]
    EmptyArgument(String),
    #[error("Invalid argument provided: {0}")]
    InvalidArgument(String),
    #[error("Unrecognized argument: {0}")]
    UnknownArgument(String),
    #[error("Required argument is missing: {0}")]
    MissingArgument(String),
    #[error("The encoder is not supported: {0}")]
    UnsupportedEncoder(String),
}