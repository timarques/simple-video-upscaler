mod stages;
mod frame;
mod error;
mod args;

use stages::Extract;
use stages::Upscale;
use stages::FilterDuplicates;
use stages::Progress;
use stages::Merge;
use args::Args;
use error::Error;

fn run_pipeline() -> Result<(), Error> {
    let args = Args::parse()?;
    args.print_options();
    let extract = Extract::execute(&args)?;
    let filter_duplicates = FilterDuplicates::execute(extract);
    let upscale = Upscale::execute(&args, filter_duplicates);
    let progress = Progress::execute(&args, upscale);
    Merge::execute(&args, progress)
}

fn main() {
    if let Err(error) = run_pipeline() {
        eprintln!("Error: {}", error);
    } else {
        println!("Completed!");
    }
}
