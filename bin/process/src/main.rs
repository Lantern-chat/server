use std::io;

use process::{Command, EncodingFormat, Response};
use processing::{read_image::read_image, ImageFormat, ProcessConfig};

fn task() -> Result<(), Box<dyn std::error::Error>> {
    let mut stdin = framed::FramedReader::new(io::stdin());

    let mut out = framed::FramedWriter::new(io::stdout());

    // TODO: Use stderr to send back results of operations,
    // OR use some kind of framed writer to send
    // back delimited chunks through stdout

    let mut processed_image = None;
    let mut heuristics = None;
    let mut config = ProcessConfig {
        max_height: 0,
        max_width: 0,
        max_pixels: 0,
    };

    bincode::serialize_into(out.new_message(), &Response::Ready)?;

    while stdin.next_msg()? {
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
                if stdin.next_msg()? {
                    let mut image = read_image(&mut stdin, &config, Some(length))?;

                    let p = processing::process::process_image(&mut image, config)?;

                    bincode::serialize_into(out.new_message(), &Response::Processed { preview: p.preview })?;

                    heuristics = Some(p.heuristics);
                    processed_image = Some(image);
                }
            }
            Command::Encode { format, quality } => {
                let image = match processed_image {
                    Some(ref image) => image,
                    None => continue,
                };

                let format = match format {
                    EncodingFormat::Png => ImageFormat::Png,
                    EncodingFormat::Jpeg => ImageFormat::Jpeg,
                    EncodingFormat::Avif => ImageFormat::Avif,
                };

                bincode::serialize_into(out.new_message(), &Response::Encoded)?;

                processing::encode::encode(out.new_message(), image, format, heuristics.unwrap(), quality)?;
            }
            Command::Clear => {
                processed_image = None;
                heuristics = None;
            }
        }
    }

    Ok(())
}

fn main() {
    if let Err(e) = task() {
        eprintln!("ERROR: {}", e);
    }
}
