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

    let db = state.db.read.get().await?;

    let row = db
        .query_one_cached_typed(
            || {
                use schema::*;
                use thorn::*;

                Query::select()
                    .from_table::<Files>()
                    .cols(&[Files::Mime, Files::Nonce, Files::Size])
                    .and_where(Files::Id.equals(Var::of(Files::Id)))
            },
            &[&file_id],
        )
        .await?;

    drop(db);

    let mime: Option<&str> = row.try_get(0)?;
    let nonce: i64 = row.try_get(1)?;
    let size: i32 = row.try_get(2)?;

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
        child
            .stdin
            .take()
            .ok_or(Error::InternalErrorStatic("Could not acquire child process stream"))?,
    );

    let mut output = AsyncFramedReader::new(
        child
            .stdout
            .take()
            .ok_or(Error::InternalErrorStatic("Could not acquire child process stream"))?,
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

                input
                    .write_buffered_object(&Command::ReadAndProcess { length: size as u64 })
                    .await?;

                let mut msg = input.new_message();
                tokio::io::copy(&mut file, &mut msg).await?;
                AsyncFramedWriter::dispose_msg(msg).await?;

                drop(_fs_permit);
            }
            Response::Processed(p) => {
                processed_response = Some(p);

                if let Some((format, quality)) = current {
                    input
                        .write_buffered_object(&Command::Encode { format, quality })
                        .await?;
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

                        db.execute_cached_typed(
                            || {
                                use schema::*;
                                use thorn::*;

                                Query::insert()
                                    .into::<Files>()
                                    .cols(&[
                                        Files::Id,
                                        Files::UserId,
                                        Files::Nonce,
                                        Files::Size,
                                        Files::Width,
                                        Files::Height,
                                        Files::Mime,
                                        Files::Flags,
                                        Files::Name,
                                    ])
                                    .values([
                                        Var::of(Files::Id),
                                        Var::of(Files::UserId),
                                        Var::of(Files::Nonce),
                                        Var::of(Files::Size),
                                        Var::of(Files::Width),
                                        Var::of(Files::Height),
                                        Var::of(Files::Mime),
                                        Var::of(Files::Flags),
                                        Var::of(Files::Name),
                                    ])
                            },
                            &[
                                &id,
                                &user_id,
                                &nonce,
                                &len,
                                &(width as i32),
                                &(height as i32),
                                &format!("image/{ext}"),
                                &FileFlags::COMPLETE.bits(),
                                &format!("{user_id}_asset.{ext}"),
                            ],
                        )
                        .await?;

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
                        input
                            .write_buffered_object(&Command::Encode { format, quality })
                            .await?;

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

    t.execute_cached_typed(
        || {
            use schema::*;
            use thorn::*;

            Query::insert()
                .into::<UserAssets>()
                .cols(&[UserAssets::Id, UserAssets::FileId, UserAssets::Preview])
                .values([
                    Var::of(UserAssets::Id),
                    Var::of(UserAssets::FileId),
                    Var::of(UserAssets::Preview),
                ])
        },
        &[&asset_id, &file_id, &processed.preview],
    )
    .await?;

    for (file_id, format, quality) in assets {
        use sdk::models::AssetFlags;

        let mut asset_flags = match format {
            EncodingFormat::Jpeg => AssetFlags::FORMAT_JPEG,
            EncodingFormat::Png => AssetFlags::FORMAT_PNG,
            EncodingFormat::Avif => AssetFlags::FORMAT_AVIF,
        };

        asset_flags = asset_flags
            .with_quality(quality)
            .with_alpha(processed.flags & process::HAS_ALPHA != 0);

        asset_flags = match format {
            // JPEGs cannot have alpha channels
            EncodingFormat::Jpeg => asset_flags.with_alpha(false),
            // PNGs are always lossless
            EncodingFormat::Png => asset_flags.with_quality(127),
            EncodingFormat::Avif => asset_flags,
        };

        t.execute_cached_typed(
            || {
                use schema::*;
                use thorn::*;

                Query::insert()
                    .into::<UserAssetFiles>()
                    .cols(&[UserAssetFiles::FileId, UserAssetFiles::AssetId, UserAssetFiles::Flags])
                    .values([
                        Var::of(UserAssetFiles::FileId),
                        Var::of(UserAssetFiles::AssetId),
                        Var::of(UserAssetFiles::Flags),
                    ])
            },
            &[&file_id, &asset_id, &asset_flags.bits()],
        )
        .await?;
    }

    t.commit().await?;

    Ok(asset_id)
}
