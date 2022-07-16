use process::{Command, EncodingFormat, Response};

//use std::io::Write;
use std::process::Stdio;
use tokio::process::Command as PCommand;

use tokio::fs;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut file = fs::OpenOptions::new().read(true).open("test.png").await?;

    let mut formats = vec![
        (EncodingFormat::Png, 100),
        (EncodingFormat::Jpeg, 95),
        (EncodingFormat::Jpeg, 80),
        (EncodingFormat::Jpeg, 45),
        (EncodingFormat::Avif, 95),
        (EncodingFormat::Avif, 80),
        (EncodingFormat::Avif, 45),
    ];

    let mut child = PCommand::new("../../target/debug/process.exe")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .spawn()?;

    let mut input = framed::tokio::AsyncFramedWriter::new(child.stdin.take().unwrap());
    let mut output = framed::tokio::AsyncFramedReader::new(child.stdout.take().unwrap());

    let mut current = None;

    while let Some(msg) = output.read_buffered_object().await? {
        match msg {
            Response::Ready => {
                input
                    .write_buffered_object(&Command::Initialize {
                        width: 3840 / 2,
                        height: 2160 / 2,
                        max_pixels: u32::MAX,
                    })
                    .await?;

                {
                    input
                        .write_buffered_object(&Command::ReadAndProcess {
                            length: file.metadata().await?.len(),
                        })
                        .await?;

                    let mut msg = input.new_message();
                    tokio::io::copy(&mut file, &mut msg).await?;
                    framed::tokio::AsyncFramedWriter::dispose_msg(msg).await?;
                }
            }
            Response::Processed { .. } => {
                if let Some((format, quality)) = formats.pop() {
                    input
                        .write_buffered_object(&Command::Encode { format, quality })
                        .await?;

                    current = Some((format, quality));
                }
            }
            Response::Encoded => {
                if let Some((format, quality)) = current {
                    let mut f = fs::OpenOptions::new()
                        .write(true)
                        .create(true)
                        .truncate(true)
                        .open(format!(
                            "./test_{}.{}",
                            quality,
                            match format {
                                EncodingFormat::Png => "png",
                                EncodingFormat::Jpeg => "jpeg",
                                EncodingFormat::Avif => "avif",
                            }
                        ))
                        .await?;

                    if let Some(msg) = output.next_msg().await? {
                        tokio::io::copy(msg, &mut f).await?;
                    }

                    if let Some((format, quality)) = formats.pop() {
                        input
                            .write_buffered_object(&Command::Encode { format, quality })
                            .await?;

                        current = Some((format, quality));
                    } else {
                        input.write_buffered_object(&Command::Clear).await?;

                        input.write_buffered_object(&Command::Exit).await?;
                    }
                }
            }
            Response::Error(err) => {
                println!("Error: {:?}", err);
                return Ok(());
            }
        }
    }

    Ok(())
}
