use image::{DynamicImage, ImageFormat, ImageResult};

use crate::read_image::{Image, ImageInfo};

pub struct EncodedImage {
    pub buffer: Vec<u8>,
    pub width: u32,
    pub height: u32,
    pub format: ImageFormat,
}

pub mod avif;
pub mod blurhash;
pub mod jpeg;
pub mod png;

pub fn encode(Image { image, info }: &Image, format: ImageFormat, quality: u8) -> ImageResult<EncodedImage> {
    debug_assert!(quality <= 100);

    match format {
        ImageFormat::Jpeg => self::jpeg::encode_jpeg(image, info, quality),
        ImageFormat::Png => self::png::encode_png(image, info, quality).map_err(Into::into),
        ImageFormat::Avif => self::avif::encode_avif(image, info, quality),
        _ => unimplemented!(),
    }
}
