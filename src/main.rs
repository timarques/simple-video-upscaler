mod frame;
mod error;
mod pipeline;
mod arguments;
mod video;
mod model;

use arguments::Arguments;
use pipeline::Pipeline;

fn main() {
    if let Err(error) = Arguments::parse().and_then(Pipeline::execute) {
        eprintln!("Error: {}", error);
    } else {
        println!("Completed!");
    }
}
