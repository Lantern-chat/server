use std::io;

use process::{Command, EncodingFormat, Response};
use processing::{read_image::read_image, ImageFormat, ProcessConfig};

fn task() -> Result<(), Box<dyn std::error::Error>> {
    let mut stdin = framed::FramedReader::new(io::stdin());

    let mut out = framed::FramedWriter::new(io::stdout());

    let mut processed_image = None;
    let mut heuristics = None;
    let mut config = ProcessConfig {
        max_height: 0,
        max_width: 0,
        max_pixels: 0,
    };

    out.write_object(&Response::Ready)?;

    while let Some(cmd) = stdin.read_object()? {
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
                if let Some(msg) = stdin.next_msg()? {
                    let mut image = read_image(msg, &config, Some(length))?;

                    let p = processing::process::process_image(&mut image, config)?;

                    out.write_object(&Response::Processed {
                        preview: p.preview,
                        width: image.image.width(),
                        height: image.image.height(),
                        flags: {
                            let mut flags = 0;

                            if image.image.color().has_alpha() {
                                flags |= process::HAS_ALPHA;
                            }

                            flags
                        },
                    })?;

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

                out.write_object(&Response::Encoded)?;

                out.with_msg(|msg| {
                    processing::encode::encode(msg, image, format, heuristics.unwrap(), quality)
                })?;
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
    if !process_utils::set_own_process_priority(process_utils::Priority::Idle) {
        eprintln!("Unable to set process priority");
    }

    if let Err(e) = task() {
        eprintln!("SUB ERROR: {}", e);
    }
}
