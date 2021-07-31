use std::io::Cursor;

use image::ImageFormat;
use models::Snowflake;
use schema::{flags::FileFlags, SnowflakeExt};

use crate::{
    ctrl::Error,
    filesystem::store::{CipherOptions, OpenMode},
    web::routes::api::v1::file::post::Metadata,
    ServerState,
};

use tokio::io::{AsyncReadExt, AsyncWriteExt};

pub async fn process_avatar(state: ServerState, user_id: Snowflake, file_id: Snowflake) -> Result<(), Error> {
    let read_db = state.db.read.get().await?;

    let row = read_db
        .query_opt_cached_typed(
            || {
                use schema::*;
                use thorn::*;

                Query::select()
                    .from_table::<Files>()
                    .cols(&[Files::Size, Files::Nonce, Files::Flags, Files::Mime])
                    .and_where(Files::UserId.equals(Var::of(Files::UserId)))
                    .and_where(Files::Id.equals(Var::of(Files::Id)))
                    .limit_n(1)
            },
            &[&user_id, &file_id],
        )
        .await?;

    let row = match row {
        Some(row) => row,
        None => return Err(Error::NotFound),
    };

    let size: i32 = row.try_get(0)?;
    let nonce: i64 = row.try_get(1)?;
    let flags = FileFlags::from_bits_truncate(row.try_get(2)?);
    let mime: Option<String> = row.try_get(3)?;

    if size > state.config.max_avatar_size {
        return Err(Error::RequestEntityTooLarge);
    }

    let mime = match mime {
        Some(mime) => mime,
        None => return Err(Error::MissingMime),
    };

    let cipher_options = CipherOptions {
        key: state.config.file_key,
        nonce: unsafe { std::mem::transmute([nonce, nonce]) },
    };

    let _fs_permit = state.fs_semaphore.acquire().await?;

    let mut file = state
        .fs
        .open_crypt(file_id, OpenMode::Read, &cipher_options)
        .await?;

    let mut buffer = Vec::with_capacity(size as usize);
    file.read_to_end(&mut buffer).await?;

    drop((file, _fs_permit));

    let _processing_permit = state.processing_semaphore.acquire().await?;

    let encode_state = state.clone();
    let encode_task = tokio::task::spawn_blocking(move || -> Result<_, Error> {
        use image::{imageops::FilterType, io::Reader};

        let format = match image::guess_format(&buffer) {
            Ok(format) => format,
            Err(_) => return Err(Error::InvalidImageFormat),
        };

        let (mut width, mut height) = {
            let mut reader = Reader::new(Cursor::new(&buffer));
            reader.set_format(format);

            match reader.into_dimensions() {
                Ok(dim) => dim,
                Err(_) => return Err(Error::InvalidImageFormat),
            }
        };

        if (width * height) > encode_state.config.max_avatar_pixels {
            return Err(Error::RequestEntityTooLarge);
        }

        // fast path if the file is already acceptable
        if format == ImageFormat::Png && width == height && width <= MAX_WIDTH {
            return Ok((false, buffer));
        }

        let mut reader = Reader::new(Cursor::new(&buffer));
        reader.set_format(format);

        let mut image = match reader.decode() {
            Ok(image) => image,
            Err(_) => return Err(Error::InvalidImageFormat),
        };

        // crop out the center
        if width != height {
            let mut x = 0;
            let mut y = 0;
            let mut new_width = width;
            let mut new_height = height;

            if width > height {
                x = (width - height) / 2;
                new_width = height;
            } else {
                y = (height - width) / 2;
                new_height = width;
            }

            log::trace!(
                "Cropping avatar image from {}x{} to {}x{}",
                width,
                height,
                new_width,
                new_height
            );

            image = image.crop_imm(x, y, new_width, new_height);

            width = new_width;
        }

        const MAX_WIDTH: u32 = 256;

        // shrink if necessary
        if width > MAX_WIDTH {
            log::trace!("Resizing avatar image from {}^2 to {}^2", width, MAX_WIDTH);

            image = image.resize(MAX_WIDTH, MAX_WIDTH, FilterType::Lanczos3);
        }

        match encode_png_best(image) {
            Ok(output) => Ok((true, output)),
            Err(e) => Err(Error::InternalError(e.to_string())),
        }
    });

    drop(_processing_permit);

    let (new_file, buffer) = encode_task.await??;

    let mut avatar_file_id = file_id;

    if new_file {
        let (new_file_id, nonce) = crate::ctrl::file::post::do_post_file(
            state.clone(),
            user_id,
            buffer.len() as i32,
            format!("{}_avatar.png", user_id),
            Some("image/png".to_owned()),
            None,
        )
        .await?;

        avatar_file_id = new_file_id;

        let _fs_permit = state.fs_semaphore.acquire().await?;

        let cipher_options = CipherOptions {
            key: state.config.file_key,
            nonce: unsafe { std::mem::transmute([nonce, nonce]) },
        };

        let mut file = state
            .fs
            .open_crypt(avatar_file_id, OpenMode::Write, &cipher_options)
            .await?;

        file.write_all(&buffer).await?;
        file.flush().await?;
    }

    let db = state.db.write.get().await?;

    if new_file {
        db.execute_cached_typed(
            || {
                use schema::*;
                use thorn::*;

                Query::update()
                    .table::<Files>()
                    .set(Files::Flags, Var::of(Files::Flags))
                    .and_where(Files::Id.equals(Var::of(Files::Id)))
            },
            &[&FileFlags::COMPLETE.bits(), &avatar_file_id],
        )
        .await?;
    }

    db.execute_cached_typed(
        || {
            use schema::*;
            use thorn::*;

            Query::call(Call::custom("lantern.upsert_user_avatar").args((
                Var::of(Users::Id),
                Var::of(Party::Id),
                Var::of(Files::Id),
            )))
        },
        &[&user_id, &Option::<i64>::None, &avatar_file_id],
    )
    .await?;

    Ok(())
}

fn encode_png_best(image: image::DynamicImage) -> Result<Vec<u8>, image::ImageError> {
    use image::{codecs::png, ColorType, DynamicImage, GenericImageView};

    let mut bytes = image.as_bytes();
    let (width, height) = image.dimensions();
    let mut color = image.color();

    let mut out = Vec::new();

    use png::{CompressionType, FilterType, PngEncoder};

    let p = PngEncoder::new_with_quality(&mut out, CompressionType::Best, FilterType::Paeth);
    let converted;
    match image {
        DynamicImage::ImageBgra8(_) => {
            converted = image.to_rgba8().into_raw();
            bytes = &converted;
            color = ColorType::Rgba8;
        }
        DynamicImage::ImageBgr8(_) => {
            converted = image.to_rgb8().into_raw();
            bytes = &converted;
            color = ColorType::Rgb8;
        }
        _ => {}
    }

    p.encode(&bytes, width, height, color)?;

    Ok(out)
}
