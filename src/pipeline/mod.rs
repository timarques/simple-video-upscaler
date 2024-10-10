mod extract;
mod upscale;
mod filter_duplicates;
mod progress;
mod merge;

use extract::Extract;
use upscale::Upscale;
use filter_duplicates::FilterDuplicates;
use progress::Progress;
use merge::Merge;

use crate::arguments::Arguments;
use crate::video::Video;
use crate::error::Error;

pub struct Pipeline;

impl Pipeline {
    pub fn execute(arguments: Arguments) -> Result<(), Error> {
        for (input, output) in &arguments.files {
            let video = Video::new(&arguments, input, output)?;
            if video.scale == 1 {
                println!("Skipping {}", input);
                continue
            }
            let extract = Extract::execute(&video)?;
            let filter_duplicates = FilterDuplicates::execute(extract);
            let upscale = Upscale::execute(&video, filter_duplicates);
            let progress = Progress::execute(&video, upscale);
            Merge::execute(&video, progress)?;
        }
        Ok(())
    }
}