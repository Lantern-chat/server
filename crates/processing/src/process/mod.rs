use std::io;

use image::{DynamicImage, GenericImageView, ImageBuffer};

use crate::{
    heuristic::EdgeInfo,
    read_image::{Image, ImageReadError},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProcessConfig {
    pub max_width: u32,
    pub max_height: u32,
    pub max_pixels: u32,
}

pub struct ProcessedImage {
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

pub mod imageops;

pub fn process_image(
    Image {
        ref mut image,
        ref info,
    }: &mut Image,
    config: ProcessConfig,
) -> Result<ProcessedImage, ProcessingError> {
    use image::{imageops::FilterType, math::Rect};

    let ProcessConfig {
        max_width,
        max_height,
        ..
    } = config;

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

    // TODO: **IMPORTANT**, combine crop+resize+alpha check into a single algorithm
    //
    // `.resize` allocates an intermediate f32 buffer for an entire image,
    // which could be avoid by going line-by-line. Compute the vertical filter for one line,
    // then reduce it horizontally

    if let Some(rect) = crop {
        let Rect {
            x,
            y,
            width: new_width,
            height: new_height,
        } = rect;

        log::trace!("Cropping image from {width}x{height} to {new_width}x{new_height}");

        //*image = match image {
        //    DynamicImage::ImageLuma8(ref image) => imageops::crop_and_resize(image, x, y, crop_width, crop_height, new_width, new_height)
        //}

        *image = imageops::crop_and_reduce(image, x, y, new_width, new_height);

        width = new_width;
        height = new_height;
    }

    // aspect ratio was already corrected above
    // so really both of these comparisons should be the same
    if width > max_width || height > max_height {
        log::trace!("Resizing image from {width}^{height} to {max_width}^{max_height}");
        *image = image.resize_exact(max_width, max_height, FilterType::Lanczos3);
    }

    *image = match (crate::util::actually_has_alpha(&image), image.color().has_color()) {
        (true, true) => DynamicImage::ImageRgba8(image.to_rgba8()),
        (false, true) => DynamicImage::ImageRgb8(image.to_rgb8()),
        (true, false) => DynamicImage::ImageLuma8(image.to_luma8()),
        (false, false) => DynamicImage::ImageLumaA8(image.to_luma_alpha8()),
    };

    let edge_info = match image {
        DynamicImage::ImageRgb8(ref image) => crate::heuristic::compute_edge_info(image),
        DynamicImage::ImageRgba8(ref image) => crate::heuristic::compute_edge_info(image),
        DynamicImage::ImageLuma8(ref image) => crate::heuristic::compute_edge_info(image),
        DynamicImage::ImageLumaA8(ref image) => crate::heuristic::compute_edge_info(image),
        _ => unreachable!(),
    };

    let preview = crate::encode::blurhash::gen_blurhash(&image);

    Ok(ProcessedImage { preview, edge_info })

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
