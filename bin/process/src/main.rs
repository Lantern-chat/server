use std::io::{self, BufWriter, Read};

use process::{Command, EncodingFormat};
use processing::{heuristic, read_image::read_image, ImageFormat, ProcessConfig};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut stdin = io::stdin();
    let mut stdout = io::stdout();
    let mut stderr = io::stderr();

    // TODO: Use stderr to send back results of operations,
    // OR use some kind of framed writer to sending
    // back delimited chunks through stdout

    let mut processed_image = None;
    let mut heuristics = None;
    let mut config = ProcessConfig {
        max_height: 0,
        max_width: 0,
        max_pixels: 0,
    };

    loop {
        // this is blocking
        let cmd: Command = bincode::deserialize_from(&mut stdin)?;

        match cmd {
            Command::Exit => return Ok(()),
            Command::Pause => continue,
            Command::Initialize {
                width,
                height,
                max_pixels,
            } => {
                config = ProcessConfig {
                    max_height: height,
                    max_width: width,
                    max_pixels,
                };
            }
            Command::ReadAndProcess { length } => {
                let mut image = read_image((&mut stdin).take(length), &config, Some(length))?;

                processing::process::process_image(&mut image, config)?;

                heuristics = Some(heuristic::compute_heuristics(&image.image));
                processed_image = Some(image);
            }
            Command::Encode { format, quality } => {
                let image = match processed_image {
                    Some(ref image) => image,
                    None => continue,
                };

                let out = BufWriter::new(&mut stdout);

                let format = match format {
                    EncodingFormat::Png => ImageFormat::Png,
                    EncodingFormat::Jpeg => ImageFormat::Jpeg,
                    EncodingFormat::Avif => ImageFormat::Avif,
                };

                processing::encode::encode(out, image, format, heuristics.unwrap(), quality)?;
            }
            Command::Clear => {
                processed_image = None;
                heuristics = None;
            }
        }
    }
}
