use std::io::{self, BufRead, Seek};

use image::DynamicImage;

use crate::{
    heuristic::EdgeInfo,
    read_image::{read_image, Image, ImageReadError, Limits},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProcessConfig {
    max_width: u32,
    max_height: u32,
    max_pixels: u32,
}

pub struct ProcessedImage {
    pub image: DynamicImage,
    pub preview: Option<Vec<u8>>,
    pub edge_info: EdgeInfo,
}

#[derive(Debug, thiserror::Error)]
pub enum ProcessingError {
    #[error("IO Error: {0}")]
    IOError(#[from] io::Error),

    #[error("Invalid Image Format")]
    InvalidImageFormat,

    #[error("Image Read Error")]
    ImageReadError(#[from] ImageReadError),

    #[error("Image Too Large")]
    TooLarge,

    #[error("Other: {0}")]
    Other(String),
}

pub fn process_image<R: BufRead + Seek>(
    mut source: R,
    config: ProcessConfig,
) -> Result<ProcessedImage, ProcessingError> {
    use image::{imageops::FilterType, math::Rect};

    let mut any_magic_bytes = Vec::with_capacity(32);
    source.read_until(32, &mut any_magic_bytes)?;

    let format = match image::guess_format(&any_magic_bytes) {
        Ok(format) => format,
        Err(_) => return Err(ProcessingError::InvalidImageFormat),
    };

    let ProcessConfig {
        max_width,
        max_height,
        max_pixels,
    } = config;

    let Image { mut image, info } = read_image(source, format, &Limits { max_pixels })?;

    let mut width = info.width;
    let mut height = info.height;

    let mut crop = None;

    if max_width == max_height {
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
    } else {
        let desired_aspect = max_width as f32 / max_height as f32;
        let actual_aspect = width as f32 / height as f32;
        let aspect_diff = desired_aspect - actual_aspect;

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

    image = match crate::util::actually_has_alpha(&image) {
        true => DynamicImage::ImageRgba8(image.to_rgba8()),
        false => DynamicImage::ImageRgb8(image.to_rgb8()),
    };

    let edge_info = match image {
        DynamicImage::ImageRgb8(ref image) => crate::heuristic::compute_edge_info(image),
        DynamicImage::ImageRgba8(ref image) => crate::heuristic::compute_edge_info(image),
        _ => unreachable!(),
    };

    let preview = crate::encode::blurhash::gen_blurhash(&image);

    Ok(ProcessedImage {
        image,
        preview,
        edge_info,
    })

    // let configs = &[
    //     (ImageFormat::Png, 100u8),
    //     (ImageFormat::Jpeg, 95),
    //     (ImageFormat::Jpeg, 70),
    //     (ImageFormat::Jpeg, 45),
    //     (ImageFormat::Avif, 99),
    //     (ImageFormat::Avif, 80),
    // ];

    // let mut images = Vec::new();

    // for &(f, mut q) in configs {
    //     if !edge_info.use_lossy() {
    //         q = q.saturating_add(5u8).min(100);
    //     }

    //     images.push(match crate::encode::encode(&image, &info, f, q) {
    //         Ok(img) => img,
    //         Err(e) => return Err(ProcessingError::Other(e.to_string())),
    //     });
    // }

    // Ok(ProcessedImages { images, preview })
}
