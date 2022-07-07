use std::io;

use image::{math::Rect, DynamicImage, GenericImageView};

use crate::{
    heuristic::HeuristicsInfo,
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
    pub heuristics: HeuristicsInfo,
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

fn compute_crop((width, height): (u32, u32), config: ProcessConfig) -> (Rect, Rect) {
    let ProcessConfig {
        max_width,
        max_height,
        ..
    } = config;

    let uncropped = Rect {
        x: 0,
        y: 0,
        width,
        height,
    };

    let mut cropped = uncropped;

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

            cropped = Rect {
                x,
                y,
                width: new_width,
                height: new_height,
            };
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

            cropped = Rect {
                x,
                y: 0,
                width: new_width,
                height: new_height,
            };
        }
    }

    (cropped, uncropped)
}

pub fn process_image(
    Image { ref mut image, .. }: &mut Image,
    config: ProcessConfig,
) -> Result<ProcessedImage, ProcessingError> {
    let ProcessConfig {
        max_width,
        max_height,
        ..
    } = config;

    let (width, height) = image.dimensions();
    let color = image.color();

    let (crop, uncrop) = compute_crop((width, height), config);

    let needs_crop = crop != uncrop;
    let needs_resize = crop.width > max_width || crop.height > max_height;
    let needs_reduce = color.bytes_per_pixel() / color.channel_count() != 1;

    let new_width = max_width.min(crop.width);
    let new_height = max_height.min(crop.height);

    *image = match (needs_crop, needs_resize, needs_reduce) {
        (true, false, true) => imageops::crop_and_reduce(image, crop),
        (_, true, true) => imageops::crop_and_reduce_and_resize(image, crop, new_width, new_height),
        (false, false, _) => match crate::util::actually_has_alpha(&image) {
            true => DynamicImage::ImageRgba8(image.to_rgba8()),
            false => DynamicImage::ImageRgb8(image.to_rgb8()),
        },
        (true, false, false) => {
            match image {
                DynamicImage::ImageLumaA8(image) => DynamicImage::ImageRgba8(imageops::reduce_to_u8(
                    &*image.view(crop.x, crop.y, crop.width, crop.height),
                )),
                DynamicImage::ImageLuma8(image) => DynamicImage::ImageRgb8(imageops::reduce_to_u8(
                    &*image.view(crop.x, crop.y, crop.width, crop.height),
                )),
                DynamicImage::ImageRgb8(_) | DynamicImage::ImageRgba8(_) => {
                    image.crop_imm(crop.x, crop.y, crop.width, crop.height)
                }
                // sane non-exhaustive fallback
                _ => DynamicImage::ImageRgba8(
                    image.crop_imm(crop.x, crop.y, crop.width, crop.height).to_rgba8(),
                ),
            }
        }
        (false, true, false) => match image {
            DynamicImage::ImageLuma8(image) => {
                DynamicImage::ImageRgb8(imageops::resize(&ReducedView::new(image), new_width, new_height))
            }
            DynamicImage::ImageLumaA8(image) => {
                DynamicImage::ImageRgba8(imageops::resize(&ReducedView::new(image), new_width, new_height))
            }
            DynamicImage::ImageRgb8(image) => {
                DynamicImage::ImageRgb8(imageops::resize(image, new_width, new_height))
            }
            DynamicImage::ImageRgba8(image) => {
                DynamicImage::ImageRgba8(imageops::resize(image, new_width, new_height))
            }
            // sane non-exhaustive fallback
            _ => DynamicImage::ImageRgba8(imageops::resize(image, new_width, new_height)),
        },
        (true, true, false) => match image {
            DynamicImage::ImageLuma8(image) => DynamicImage::ImageRgb8(imageops::crop_and_resize(
                &ReducedView::new(image),
                crop,
                new_width,
                new_height,
            )),
            DynamicImage::ImageLumaA8(image) => DynamicImage::ImageRgba8(imageops::crop_and_resize(
                &ReducedView::new(image),
                crop,
                new_width,
                new_height,
            )),
            DynamicImage::ImageRgb8(image) => {
                DynamicImage::ImageRgb8(imageops::crop_and_resize(image, crop, new_width, new_height))
            }
            DynamicImage::ImageRgba8(image) => {
                DynamicImage::ImageRgba8(imageops::crop_and_resize(image, crop, new_width, new_height))
            }
            // sane non-exhaustive fallback
            _ => DynamicImage::ImageRgba8(imageops::crop_and_resize(image, crop, new_width, new_height)),
        },
    };

    let heuristics = match image {
        DynamicImage::ImageRgb8(ref image) => crate::heuristic::compute_heuristics(image),
        DynamicImage::ImageRgba8(ref image) => crate::heuristic::compute_heuristics(image),
        _ => unreachable!(),
    };

    let preview = crate::encode::blurhash::gen_blurhash(image.thumbnail(64, 64));

    Ok(ProcessedImage { preview, heuristics })
}

use imageops::ReducedView;
