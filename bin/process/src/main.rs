use std::io;

use framed::{FramedReader, FramedWriter};
use process::{Command, EncodingFormat, Error, ProcessedResponse, Response};
use processing::{
    heuristic::HeuristicsInfo,
    image::DynamicImage,
    read_image::{read_image, Image},
    ImageFormat, ProcessConfig,
};

struct ProcessState {
    config: ProcessConfig,
    image: Option<Image>,
    heuristics: Option<HeuristicsInfo>,
}

enum Action {
    Exit,
    Continue,
}

impl ProcessState {
    fn run(
        &mut self,
        input: &mut FramedReader<io::Stdin>,
        output: &mut FramedWriter<io::Stdout>,
        cmd: Command,
    ) -> Result<Action, Error> {
        match cmd {
            Command::Exit => return Ok(Action::Exit),
            Command::Pause => return Ok(Action::Continue),
            Command::Initialize {
                width,
                height,
                max_pixels,
            } => {
                self.config = ProcessConfig {
                    max_height: height,
                    max_width: width,
                    max_pixels,
                };
            }
            Command::Clear => {
                self.image = None;
                self.heuristics = None;
            }
            Command::ReadAndProcess { length } => {
                if let Some(msg) = input.next_msg()? {
                    let mut image = read_image(msg, &self.config, Some(length))?;

                    let p = processing::process::process_image(&mut image, self.config)?;

                    output.write_object(&Response::Processed(ProcessedResponse {
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
                    }))?;

                    self.heuristics = Some(p.heuristics);
                    self.image = Some(image);
                }
            }
            Command::Encode { format, quality } => {
                let image = match self.image {
                    Some(ref mut image) => image,
                    None => return Ok(Action::Continue),
                };

                let format = match format {
                    EncodingFormat::Png => ImageFormat::Png,
                    EncodingFormat::Jpeg => {
                        // even if mozjpeg can handle RGBA bytes, it still needs to be
                        // premultiplied or it'll be a mess
                        if let DynamicImage::ImageRgba8(ref mut rgba) = image.image {
                            processing::process::imageops::fast_premultiply_alpha(rgba);
                        }

                        ImageFormat::Jpeg
                    }
                    EncodingFormat::Avif => ImageFormat::Avif,
                };

                output.write_object(&Response::Encoded)?;

                output.with_msg(|msg| {
                    processing::encode::encode(msg, image, format, self.heuristics.unwrap(), quality)
                })?;
            }
        }

        Ok(Action::Continue)
    }
}

fn task() -> Result<(), Box<dyn std::error::Error>> {
    let mut input = FramedReader::new(io::stdin());
    let mut output = FramedWriter::new(io::stdout());

    let mut state = ProcessState {
        config: ProcessConfig {
            max_width: 0,
            max_height: 0,
            max_pixels: 0,
        },
        image: None,
        heuristics: None,
    };

    output.write_object(&Response::Ready)?;

    while let Some(cmd) = input.read_object()? {
        match state.run(&mut input, &mut output, cmd) {
            Ok(action) => match action {
                Action::Continue => continue,
                Action::Exit => return Ok(()),
            },
            Err(e) => output.write_object(&Response::Error(e))?,
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
