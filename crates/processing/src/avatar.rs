use std::io::Cursor;

pub struct ProcessConfig {
    pub max_width: u32,
    pub max_pixels: u32,
}

pub struct ProcessedImage {
    pub reused: bool,
    pub image: EncodedImage,
}

pub enum ProcessingError {
    InvalidImageFormat,
    TooLarge,
    Other(String),
}

pub fn process_avatar(
    buffer: Vec<u8>,
    preview: Option<Vec<u8>>,
    config: ProcessConfig,
) -> Result<ProcessedImage, ProcessingError> {
    use image::{imageops::FilterType, io::Reader, ImageFormat};

    let format = match image::guess_format(&buffer) {
        Ok(format) => format,
        Err(_) => return Err(ProcessingError::InvalidImageFormat),
    };

    let (mut width, height) = {
        let mut reader = Reader::new(Cursor::new(&buffer));
        reader.set_format(format);

        match reader.into_dimensions() {
            Ok(dim) => dim,
            Err(_) => return Err(ProcessingError::InvalidImageFormat),
        }
    };

    if (width * height) > config.max_pixels {
        return Err(ProcessingError::TooLarge);
    }

    let max_width = config.max_width;

    let try_use_existing = format == ImageFormat::Png && width == height && width <= max_width;

    let mut reader = Reader::new(Cursor::new(&buffer));
    reader.set_format(format);

    let mut image = match reader.decode() {
        Ok(image) => image,
        Err(_) => return Err(ProcessingError::InvalidImageFormat),
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

            Ok(ProcessedImage {
                reused,
                image: output,
            })
        }
        Err(e) => Err(ProcessingError::Other(e.to_string())),
    }
}

pub struct EncodedImage {
    pub buffer: Vec<u8>,
    pub preview: Option<Vec<u8>>,
}

fn encode_png_best(
    mut image: image::DynamicImage,
    mut preview: Option<Vec<u8>>,
) -> Result<EncodedImage, image::ImageError> {
    use image::{codecs::png, DynamicImage, GenericImageView};
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
