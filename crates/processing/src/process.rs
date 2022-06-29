use crate::read_image::{read_image, Image, ImageInfo, ImageReadError, Limits};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProcessConfig {
    Avatar {
        max_width: u32,
        max_pixels: u32,
    },
    Banner {
        max_width: u32,
        max_height: u32,
        max_pixels: u32,
    },
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

pub fn process_image(
    buffer: Vec<u8>,
    preview: Option<Vec<u8>>,
    config: ProcessConfig,
) -> Result<ProcessedImage, ProcessingError> {
    use image::{imageops::FilterType, ImageFormat};

    let format = match image::guess_format(&buffer) {
        Ok(format) => format,
        Err(_) => return Err(ProcessingError::InvalidImageFormat),
    };

    let (max_width, max_height, max_pixels) = match config {
        ProcessConfig::Avatar {
            max_pixels,
            max_width,
        } => (max_width, max_width, max_pixels),
        ProcessConfig::Banner {
            max_pixels,
            max_width,
            max_height,
        } => (max_width, max_height, max_pixels),
    };

    let Image { mut image, info } = read_image(&buffer, format, &Limits { max_pixels })?;

    let mut width = info.width;
    let mut height = info.height;

    let mut try_use_existing = format == ImageFormat::Png && width <= max_width && height <= max_height;

    match config {
        ProcessConfig::Avatar { .. } => {
            try_use_existing &= width == height;

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
                height = new_height;
            }
        }
        ProcessConfig::Banner { .. } => {
            let desired_aspect = max_width as f32 / max_height as f32;
            let actual_aspect = width as f32 / height as f32;
            let aspect_diff = desired_aspect - actual_aspect;

            // For example, 16/9 > 16/10
            try_use_existing &= 0.0 <= aspect_diff && aspect_diff < 0.4; // allow slight overhang that's cropped client-side

            // crop if not ideal
            if aspect_diff.abs() > 0.01 {
                let mut x = 0;
                let mut new_width = width;
                let mut new_height = height;

                if aspect_diff > 0.0 {
                    // image is taller than needed
                    new_height = width * max_height / max_width;
                } else {
                    // image is wider than needed
                    new_width = height * max_width / max_height;
                    x = (width - new_width) / 2; // center horizontally
                }

                log::trace!("Cropping banner image from {width}x{height} to {new_width}x{new_height}");

                image = image.crop_imm(x, 0, new_width, new_height);

                width = new_width;
                height = new_height;
            }
        }
    }

    // aspect ratio was already corrected above
    // so really both of these comparisons should be the same
    if width > max_width || height > max_height {
        log::trace!("Resizing avatar or banner image from {width}^{height} to {max_width}^{max_height}");
        image = image.resize(max_width, max_height, FilterType::Lanczos3);
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
