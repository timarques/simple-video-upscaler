mod arguments;
mod frame;
mod model;
mod pipeline;
mod reorderer;
mod upscaler;
mod video;

use arguments::Arguments;
use pipeline::Pipeline;

fn main() {
    if let Err(error) = Arguments::parse().and_then(Pipeline::execute) {
        eprintln!("Error: {error}");
    } else {
        println!("Completed!");
    }
}
