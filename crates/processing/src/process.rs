use crate::{
    encode::EncodedImage,
    read_image::{read_image, Image, ImageReadError, Limits},
};

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
    use image::{imageops::FilterType, math::Rect, ImageFormat};

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

    let mut try_use_existing = matches!(format, ImageFormat::Png | ImageFormat::Jpeg | ImageFormat::Gif)
        && width <= max_width
        && height <= max_height;

    let mut crop = None;

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

                crop = Some(Rect {
                    x,
                    y,
                    width: new_width,
                    height: new_height,
                });
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

                crop = Some(Rect {
                    x,
                    y: 0,
                    width: new_width,
                    height: new_height,
                });
            }
        }
    }

    if let Some(rect) = crop {
        let Rect {
            x,
            y,
            width: new_width,
            height: new_height,
        } = rect;

        log::trace!("Cropping image from {width}x{height} to {new_width}x{new_height}");

        image = image.crop_imm(x, y, new_width, new_height);
        width = new_width;
        height = new_height;
    }

    // aspect ratio was already corrected above
    // so really both of these comparisons should be the same
    if width > max_width || height > max_height {
        log::trace!("Resizing image from {width}^{height} to {max_width}^{max_height}");
        image = image.resize(max_width, max_height, FilterType::Lanczos3);
    }

    // encode the image and generate a blurhash preview if needed
    match crate::encode::encode_png_best(image, preview, info) {
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
