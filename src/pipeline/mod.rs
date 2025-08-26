mod extract;
mod upscale;
mod progress;
mod merge;

use self::extract::ExtractPhase;
use self::upscale::UpscalePhase;
use self::progress::ProgressPhase;
use self::merge::MergePhase;

use crate::arguments::Arguments;
use crate::video::Video;
use anyhow::Result;

const CHANNEL_COUNT: usize = 4;

pub struct Pipeline;

impl Pipeline {

    pub fn execute(arguments: Arguments) -> Result<()> {
        for (input, output) in &arguments.files {
            let video = Video::new(&arguments, input, output)?;
            if video.model.is_none() {
                continue
            }
            
            let extract = ExtractPhase::execute(&video)?;
            let upscale = UpscalePhase::execute(&video, extract)?;
            let merge = MergePhase::execute(&video, upscale)?;
            ProgressPhase::execute(&video, merge);
        }
        Ok(())
    }
}