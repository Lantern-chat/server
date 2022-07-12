use process::{Command, EncodingFormat, Response};

use std::io::Write;
use std::process::{Command as PCommand, Stdio};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let file = std::fs::read("test.png")?;

    let mut child = PCommand::new("../../target/debug/process.exe")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .spawn()?;

    let mut input = framed::FramedWriter::new(child.stdin.take().unwrap());
    let mut output = framed::FramedReader::new(child.stdout.take().unwrap());
    //let mut err = child.stderr.take().unwrap();

    while let Some(msg) = output.read_object()? {
        match msg {
            Response::Ready => {
                input.write_object(&Command::Initialize {
                    width: 1600,
                    height: 900,
                    max_pixels: u32::MAX,
                })?;

                {
                    input.write_object(&Command::ReadAndProcess {
                        length: file.len() as u64,
                    })?;
                    input.new_message().write_all(&file)?;
                }
            }
            Response::Processed { .. } => {
                input.write_object(&Command::Encode {
                    format: EncodingFormat::Jpeg,
                    quality: 100,
                })?;
            }
            Response::Encoded => {
                let mut f = std::fs::OpenOptions::new()
                    .write(true)
                    .create(true)
                    .truncate(true)
                    .open("./out.jpeg")?;

                if let Some(msg) = output.next_msg()? {
                    std::io::copy(msg, &mut f)?;
                }

                input.write_object(&Command::Exit)?;
            }
        }
    }

    Ok(())
}
