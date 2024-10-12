# Simple Video Upscaler

This tool uses `RealCugan` for video upscaling, utilizing `ffmpeg` to process video files.
It allows users to input a video, specify target resolution, and output format using a specified encoder.

## Usage

### Command-line Options

**Usage: simple_upscaler [OPTIONS]**

#### Options:
- -i, --input FILE/DIRECTORY Input video file
- -o, --output FILE/DIRECTORY Output video file
- -w, --width WIDTH Target width (optional)
- -h, --height HEIGHT Target height (optional)
- -e, --encoder ENCODER Video encoder (default: libx264)
- -s, --scale SCALE Video scale factor(default: 2)
- --help Show this help message

## Requirements

- ffmpeg
- ffprobe