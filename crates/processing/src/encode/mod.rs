use std::io::Write;

use image::{ImageFormat, ImageResult};

use crate::{heuristic::HeuristicsInfo, read_image::Image};

pub mod avif;
pub mod blurhash;
pub mod jpeg;
pub mod png;

pub fn encode<W: Write>(
    w: W,
    Image { image, info }: &Image,
    format: ImageFormat,
    heuristics: HeuristicsInfo,
    quality: u8,
) -> ImageResult<()> {
    debug_assert!(quality <= 100);

    match format {
        ImageFormat::Jpeg => self::jpeg::encode_jpeg(w, image, info, heuristics, quality),
        ImageFormat::Png => self::png::encode_png(w, image, info, quality).map_err(Into::into),
        ImageFormat::Avif => self::avif::encode_avif(w, image, info, heuristics, quality),
        _ => unimplemented!(),
    }
}
