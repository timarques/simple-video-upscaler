mod extract;
mod merge;
mod progress;
mod upscale;

use self::extract::ExtractPhase;
use self::merge::MergePhase;
use self::progress::ProgressPhase;
use self::upscale::UpscalePhase;

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
                continue;
            }

            let extract = ExtractPhase::execute(&video)?;
            let upscale = UpscalePhase::execute(&video, extract)?;
            let merge = MergePhase::execute(&video, upscale)?;
            ProgressPhase::execute(&video, merge);
        }
        Ok(())
    }
}
