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
                    .cols(&[
                        Files::Size,
                        Files::Nonce,
                        Files::Flags,
                        Files::Mime,
                        Files::Preview,
                    ])
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

    if size > state.config.max_avatar_size {
        return Err(Error::RequestEntityTooLarge);
    }

    let nonce: i64 = row.try_get(1)?;
    let flags = FileFlags::from_bits_truncate(row.try_get(2)?);
    let mime: Option<String> = row.try_get(3)?;

    // TODO: Use mime type somehow?
    let _mime = match mime {
        Some(mime) => mime,
        None => return Err(Error::MissingMime),
    };

    let preview: Option<Vec<u8>> = row.try_get(4)?;

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

        let (mut width, height) = {
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

        let max_width = encode_state.config.max_avatar_width;

        let try_use_existing = format == ImageFormat::Png && width == height && width <= max_width;

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

        // shrink if necessary
        if width > max_width {
            log::trace!("Resizing avatar image from {}^2 to {}^2", width, max_width);

            image = image.resize(max_width, max_width, FilterType::Lanczos3);
        }

        // encode the image and generate a blurhash preview if needed
        match encode_png_best(image, preview) {
            Ok(mut output) => {
                let mut reused = false;

                // if the existing PNG buffer is somehow smaller, use that
                // could happen with a very-optimized PNG from photoshop or something
                if try_use_existing && buffer.len() < output.buffer.len() {
                    log::trace!(
                        "PNG Encoder got worse compression than original, {} vs {}",
                        buffer.len(),
                        output.buffer.len()
                    );

                    reused = true;
                    output.buffer = buffer;
                }

                Ok((!reused, output))
            }
            Err(e) => Err(Error::InternalError(e.to_string())),
        }
    });

    let (new_file, encoded_image) = encode_task.await??;

    drop(_processing_permit);

    let mut avatar_file_id = file_id;

    if new_file {
        let (new_file_id, nonce) = crate::ctrl::file::post::do_post_file(
            state.clone(),
            user_id,
            encoded_image.buffer.len() as i32,
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

        file.write_all(&encoded_image.buffer).await?;
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
                    .set(Files::Preview, Var::of(Files::Preview))
                    .and_where(Files::Id.equals(Var::of(Files::Id)))
            },
            &[
                &FileFlags::COMPLETE.bits(),
                &encoded_image.preview,
                &avatar_file_id,
            ],
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

struct EncodedImage {
    buffer: Vec<u8>,
    preview: Option<Vec<u8>>,
}

fn encode_png_best(
    mut image: image::DynamicImage,
    mut preview: Option<Vec<u8>>,
) -> Result<EncodedImage, image::ImageError> {
    use image::{codecs::png, ColorType, DynamicImage, GenericImageView};
    use png::{CompressionType, FilterType, PngEncoder};

    image = match image {
        DynamicImage::ImageBgra8(_) => DynamicImage::ImageRgba8(image.to_rgba8()),
        DynamicImage::ImageBgr8(_) => DynamicImage::ImageRgb8(image.to_rgb8()),
        _ => image,
    };

    let bytes = image.as_bytes();
    let (width, height) = image.dimensions();
    let color = image.color();

    // 1.5 bytes per pixel
    const BYTES_PER_PIXEL_D: usize = 3;
    const BYTES_PER_PIXEL_N: usize = 2;

    let expected_bytes = ((width * height) as usize * BYTES_PER_PIXEL_D) / BYTES_PER_PIXEL_N;

    let mut out = Vec::with_capacity(expected_bytes);

    PngEncoder::new_with_quality(&mut out, CompressionType::Best, FilterType::Paeth)
        .encode(&bytes, width, height, color)?;

    if preview.is_none() {
        let has_alpha = image.color().has_alpha();

        let mut bytes = match has_alpha {
            true => image.to_rgba8().into_raw(),
            false => image.to_rgb8().into_raw(),
        };

        let (xc, yc) = blurhash::encode::num_components(width, height);

        // encode routine automatically premultiplies alpha
        let hash = blurhash::encode::encode(
            xc,
            yc,
            width as usize,
            height as usize,
            &mut bytes,
            if has_alpha { 4 } else { 3 },
        );

        match hash {
            Err(e) => {
                log::error!("Error computing blurhash for avatar: {}", e)
            }
            Ok(hash) => {
                preview = Some(hash);
            }
        }
    }

    log::trace!(
        "PNG Encoder expected {} bytes, got {} bytes",
        expected_bytes,
        out.len()
    );

    Ok(EncodedImage { buffer: out, preview })
}
