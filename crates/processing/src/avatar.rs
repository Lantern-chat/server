use crate::read_image::{read_image, Image, ImageInfo, ImageReadError, Limits};

pub struct ProcessConfig {
    pub max_width: u32,
    pub max_pixels: u32,
}

pub struct ProcessedImage {
    pub reused: bool,
    pub image: EncodedImage,
}

#[derive(Debug, thiserror::Error)]
pub enum ProcessingError {
    #[error("Invalid Image Format")]
    InvalidImageFormat,

    #[error("Image Read Error")]
    ImageReadError(#[from] ImageReadError),

    #[error("Image Too Large")]
    TooLarge,

    #[error("Other: {0}")]
    Other(String),
}

pub fn process_avatar(
    buffer: Vec<u8>,
    preview: Option<Vec<u8>>,
    config: ProcessConfig,
) -> Result<ProcessedImage, ProcessingError> {
    use image::{imageops::FilterType, ImageFormat};

    let format = match image::guess_format(&buffer) {
        Ok(format) => format,
        Err(_) => return Err(ProcessingError::InvalidImageFormat),
    };

    let Image { mut image, info } = read_image(
        &buffer,
        format,
        &Limits {
            max_pixels: config.max_pixels,
        },
    )?;

    let max_width = config.max_width;

    let mut width = info.width;
    let height = info.height;

    let try_use_existing = format == ImageFormat::Png && width == height && width <= max_width;

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

        log::trace!("Cropping avatar image from {width}x{height} to {new_width}x{new_height}");

        image = image.crop_imm(x, y, new_width, new_height);

        width = new_width;
    }

    // shrink if necessary
    if width > max_width {
        log::trace!("Resizing avatar image from {width}^2 to {max_width}^2");

        image = image.resize(max_width, max_width, FilterType::Lanczos3);
    }

    // encode the image and generate a blurhash preview if needed
    match encode_png_best(image, preview, info) {
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
    pub width: u32,
    pub height: u32,
}

#[allow(unused_imports, unused_variables)]
fn encode_png_best(
    mut image: image::DynamicImage,
    mut preview: Option<Vec<u8>>,
    info: ImageInfo,
) -> std::io::Result<EncodedImage> {
    use image::{ColorType, DynamicImage, GenericImageView};
    use png::{
        AdaptiveFilterType, BitDepth, Compression, Encoder as PngEncoder, FilterType, ScaledFloat,
        SourceChromaticities, SrgbRenderingIntent,
    };

    image = match image {
        DynamicImage::ImageRgba16(_) => DynamicImage::ImageRgba8(image.to_rgba8()),
        DynamicImage::ImageRgb16(_) => DynamicImage::ImageRgb8(image.to_rgb8()),
        DynamicImage::ImageLuma16(_) => DynamicImage::ImageLuma8(image.to_luma8()),
        DynamicImage::ImageLumaA16(_) => DynamicImage::ImageLumaA8(image.to_luma_alpha8()),
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

    let mut encoder = PngEncoder::new(&mut out, width, height);

    encoder.set_depth(BitDepth::Eight);
    encoder.set_color(match color {
        ColorType::L8 => png::ColorType::Grayscale,
        ColorType::La8 => png::ColorType::GrayscaleAlpha,
        ColorType::Rgb8 => png::ColorType::Rgb,
        ColorType::Rgba8 => png::ColorType::Rgba,
        _ => unreachable!(),
    });

    //encoder.set_trns(&[0xFFu8, 0xFFu8, 0xFFu8, 0xFFu8] as &'static [u8]);
    encoder.set_compression(Compression::Best);
    encoder.set_filter(FilterType::Paeth);
    encoder.set_adaptive_filter(AdaptiveFilterType::Adaptive);

    //encoder.set_srgb(info.srgb.unwrap_or(SrgbRenderingIntent::AbsoluteColorimetric));
    //encoder.set_source_gamma(info.source_gamma.unwrap_or_else(|| {
    //    // Value taken from https://www.w3.org/TR/2003/REC-PNG-20031110/#11sRGB
    //    ScaledFloat::from_scaled(45455)
    //}));
    //encoder.set_source_chromaticities(info.source_chromaticities.unwrap_or_else(|| {
    //    // Values taken from https://www.w3.org/TR/2003/REC-PNG-20031110/#11sRGB
    //    SourceChromaticities {
    //        white: (ScaledFloat::from_scaled(31270), ScaledFloat::from_scaled(32900)),
    //        red: (ScaledFloat::from_scaled(64000), ScaledFloat::from_scaled(33000)),
    //        green: (ScaledFloat::from_scaled(30000), ScaledFloat::from_scaled(60000)),
    //        blue: (ScaledFloat::from_scaled(15000), ScaledFloat::from_scaled(6000)),
    //    }
    //}));
    //if let Some(_icc_profile) = info.icc_profile {
    //    // TODO: ICC Profile
    //}

    let mut writer = encoder.write_header()?;
    writer.write_image_data(bytes)?;

    drop(writer);

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
                log::error!("Error computing blurhash for avatar: {e}")
            }
            Ok(hash) => {
                preview = Some(hash);
            }
        }
    }

    log::trace!(
        "PNG Encoder expected {expected_bytes} bytes, got {} bytes",
        out.len()
    );

    Ok(EncodedImage {
        buffer: out,
        preview,
        width,
        height,
    })
}
