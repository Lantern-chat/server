use filesystem::store::{CipherOptions, OpenMode};
use framed::tokio::{AsyncFramedReader, AsyncFramedWriter};
use process::{Command, EncodingFormat, ProcessedResponse, Response};
use rand::Rng;

//use std::io::Write;
use std::process::Stdio;
use tokio::process::Command as SystemCommand;

use schema::{flags::FileFlags, SnowflakeExt};
use sdk::Snowflake;

use crate::{Error, ServerState};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AssetMode {
    Avatar,
    Banner,
}

// TODO: Support animated formats

fn gen_formats(state: &ServerState, mode: AssetMode) -> Vec<(EncodingFormat, u8)> {
    let mut all_formats = Vec::new();

    let config = state.config();
    let formats = match mode {
        AssetMode::Avatar => &config.upload.avatar_formats,
        AssetMode::Banner => &config.upload.banner_formats,
    };

    // NOTE: JPEG must go at bottom/last, as it will premultiply any alpha channel
    // NOTE 2: The formats are iterated on via .pop(), so it goes in reverse.
    all_formats.extend(formats.jpeg.iter().copied().map(|q| (EncodingFormat::Jpeg, q)));

    all_formats.extend(formats.avif.iter().copied().map(|q| (EncodingFormat::Avif, q)));
    all_formats.extend(formats.png.iter().copied().map(|q| (EncodingFormat::Png, q)));

    all_formats
}

pub async fn add_asset(
    state: &ServerState,
    mode: AssetMode,
    user_id: Snowflake,
    file_id: Snowflake,
) -> Result<Snowflake, Error> {
    let max_width;
    let max_height;
    let max_pixels;
    let max_size;

    let config = state.config();
    match mode {
        AssetMode::Avatar => {
            let width = config.upload.avatar_width;
            max_width = width;
            max_height = width;
            max_pixels = config.upload.max_avatar_pixels;
            max_size = config.upload.max_avatar_size;
        }
        AssetMode::Banner => {
            max_width = config.upload.banner_width;
            max_height = config.upload.banner_height;
            max_pixels = config.upload.max_banner_pixels;
            max_size = config.upload.max_banner_size;
        }
    }

    #[rustfmt::skip]
    let row = state.db.read.get().await?.query_one2(schema::sql! {
        SELECT Files.Size AS @Size, Files.Nonce AS @Nonce, Files.Mime AS @Mime
        FROM Files WHERE Files.Id = #{&file_id as Files::Id}
    }?).await?;

    let size: i32 = row.size()?;
    let nonce: i64 = row.nonce()?;
    let mime: Option<&str> = row.mime()?;

    if let Some(mime) = mime {
        if !mime.starts_with("image") {
            return Err(Error::InvalidImageFormat);
        }
    }

    if size > max_size {
        return Err(Error::RequestEntityTooLarge);
    }

    let _cpu_permit = state.cpu_semaphore.acquire().await?;

    let mut child = SystemCommand::new(state.config().paths.bin_path.join("process"))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .kill_on_drop(true)
        .spawn()?;

    let mut input = AsyncFramedWriter::new(
        child.stdin.take().ok_or(Error::InternalErrorStatic("Could not acquire child process stream"))?,
    );

    let mut output = AsyncFramedReader::new(
        child.stdout.take().ok_or(Error::InternalErrorStatic("Could not acquire child process stream"))?,
    );

    let mut formats = gen_formats(state, mode);
    let mut current = formats.pop();
    let mut processed_response = None;

    let mut assets = Vec::new();

    while let Some(msg) = output.read_buffered_object().await? {
        match msg {
            Response::Ready => {
                input
                    .write_buffered_object(&Command::Initialize {
                        width: max_width,
                        height: max_height,
                        max_pixels,
                    })
                    .await?;

                let _fs_permit = state.fs_semaphore.acquire().await?;

                let mut file = state
                    .fs()
                    .open_crypt(
                        file_id,
                        OpenMode::Read,
                        &CipherOptions::new_from_i64_nonce(state.config().keys.file_key, nonce),
                    )
                    .await?;

                input.write_buffered_object(&Command::ReadAndProcess { length: size as u64 }).await?;

                let mut msg = input.new_message();
                tokio::io::copy(&mut file, &mut msg).await?;
                AsyncFramedWriter::dispose_msg(msg).await?;

                drop(_fs_permit);
            }
            Response::Processed(p) => {
                processed_response = Some(p);

                if let Some((format, quality)) = current {
                    input.write_buffered_object(&Command::Encode { format, quality }).await?;
                }
            }
            Response::Error(e) => {
                use process::Error as P;
                return Err(match e {
                    P::FileTooLarge | P::ImageTooLarge => Error::RequestEntityTooLarge,
                    P::InvalidImageFormat | P::UnsupportedFormat | P::DecodingError(_) => {
                        Error::InvalidImageFormat
                    }
                    _ => Error::InternalError(e.to_string()),
                });
            }
            Response::Encoded => {
                if let Some((format, quality)) = current {
                    let nonce: i64 = util::rng::crypto_thread_rng().gen();
                    let id = Snowflake::now();

                    let _fs_permit = state.fs_semaphore.acquire().await?;

                    let mut file = state
                        .fs()
                        .open_crypt(
                            id,
                            OpenMode::Write,
                            &CipherOptions::new_from_i64_nonce(state.config().keys.file_key, nonce),
                        )
                        .await?;

                    // put all of the file-writing in a try-block so if an error occurs then
                    // the file can be immediately deleted
                    let do_file = async {
                        let Some(msg) = output.next_msg().await? else {
                            return Err(Error::InternalErrorStatic("Failed to read encoded image"));
                        };

                        let len = file.copy_from(msg).await? as i32;
                        drop(_fs_permit);

                        let db = state.db.write.get().await?;

                        let Some(ref processed) = processed_response else {
                            return Err(Error::InternalErrorStatic("Never received processed image"));
                        };

                        let ProcessedResponse { width, height, .. } = *processed;

                        let ext = match format {
                            EncodingFormat::Jpeg => "jpeg",
                            EncodingFormat::Png => "png",
                            EncodingFormat::Avif => "avif",
                        };

                        let width = width as i32;
                        let height = height as i32;
                        let mime = format!("image/{ext}");
                        let flags = FileFlags::COMPLETE.bits();
                        let name = format!("{user_id}_asset.{ext}");

                        #[rustfmt::skip]
                        db.execute2(schema::sql! {
                            INSERT INTO Files (Id, UserId, Nonce, Size, Width, Height, Mime, Flags, Name)
                            VALUES (
                                #{&id       as Files::Id},
                                #{&user_id  as Files::UserId},
                                #{&nonce    as Files::Nonce},
                                #{&len      as Files::Size},
                                #{&width    as Files::Width},
                                #{&height   as Files::Height},
                                #{&mime     as Files::Mime},
                                #{&flags    as Files::Flags},
                                #{&name     as Files::Name}
                            )
                        }?).await?;

                        assets.push((id, format, quality));

                        Ok(())
                    };

                    if let Err(e) = do_file.await {
                        // if there was an error here, delete the file
                        if let Err(e2) = state.fs().delete(id).await {
                            log::error!("Error deleting file after encoding/database error: {}", e2);
                        }

                        return Err(e);
                    }

                    current = formats.pop();

                    if let Some((format, quality)) = current {
                        input.write_buffered_object(&Command::Encode { format, quality }).await?;

                        continue;
                    } else {
                        input.write_buffered_object(&Command::Exit).await?;

                        break;
                    }
                }
            }
        }
    }

    drop(child);
    drop(_cpu_permit);

    let Some(processed) = processed_response else {
        return Err(Error::InternalErrorStatic("Never received processed image"));
    };

    let asset_id = Snowflake::now();

    let mut db = state.db.write.get().await?;
    let t = db.transaction().await?;

    t.execute2(schema::sql! {
        INSERT INTO UserAssets (Id, FileId, Preview)
        VALUES (
            #{&asset_id as UserAssets::Id },
            #{&file_id as UserAssets::FileId },
            #{&processed.preview as UserAssets::Preview }
        )
    }?)
    .await?;

    for (file_id, format, quality) in assets {
        use sdk::models::AssetFlags;

        let mut asset_flags = match format {
            EncodingFormat::Jpeg => AssetFlags::FORMAT_JPEG,
            EncodingFormat::Png => AssetFlags::FORMAT_PNG,
            EncodingFormat::Avif => AssetFlags::FORMAT_AVIF,
        };

        asset_flags = asset_flags.with_quality(quality).with_alpha(processed.flags & process::HAS_ALPHA != 0);

        asset_flags = match format {
            // JPEGs cannot have alpha channels
            EncodingFormat::Jpeg => asset_flags.with_alpha(false),
            // PNGs are always lossless
            EncodingFormat::Png => asset_flags.with_quality(127),
            EncodingFormat::Avif => asset_flags,
        };

        let asset_flags = asset_flags.bits();

        t.execute2(schema::sql! {
            INSERT INTO UserAssetFiles (FileId, AssetId, Flags)
            VALUES (
                #{&file_id as UserAssetFiles::FileId},
                #{&asset_id as UserAssetFiles::AssetId},
                #{&asset_flags as UserAssetFiles::Flags}
            )
        }?)
        .await?;
    }

    t.commit().await?;

    Ok(asset_id)
}
