# Simple Video Upscaler

This tool uses `RealCugan` for video upscaling, utilizing `ffmpeg` to process video files.
It allows users to input a video, specify target resolution, and output format using a specified encoder.

## Usage

### Command-line Options

**Usage: simple_upscaler [OPTIONS]**

#### Options:
- -i, --input FILE Input video file
- -o, --output FILE Output video file
- --width WIDTH Target width (optional)
- --height HEIGHT Target height (optional)
- --encoder ENCODER Video encoder (default: libx264)
- -h, --help Show this help message

## Requirements

- ffmpeg
- ffprobe