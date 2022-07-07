use image::{DynamicImage, GenericImageView, ImageFormat, ImageResult};

use crate::{heuristic::HeuristicsInfo, read_image::ImageInfo};

use super::EncodedImage;

#[cfg(test)]
fn map_jpeg_to_avif_quality(q: u8) -> u8 {
    if q == 100 || q == 0 {
        return q;
    }

    const A: f32 = 1.0 / 55.0;
    const C: f32 = 39.5825619849;
    //const C: f32 = 100.0 / (2f32.powf(100.0 / A) - 1.0);

    C.mul_add(2f32.powf(q as f32 * A), -C) as u8
}

/// `map_jpeg_to_avif_quality` made into a lookup table
static JPEG_TO_AVIF_QUALITY: [u8; 101] = [
    0, 0, 1, 1, 2, 2, 3, 3, 4, 4, 5, 5, 6, 7, 7, 8, 8, 9, 10, 10, 11, 11, 12, 13, 13, 14, 15, 16, 16, 17, 18,
    18, 19, 20, 21, 21, 22, 23, 24, 25, 25, 26, 27, 28, 29, 30, 31, 31, 32, 33, 34, 35, 36, 37, 38, 39, 40,
    41, 42, 43, 44, 45, 46, 47, 49, 50, 51, 52, 53, 54, 56, 57, 58, 59, 61, 62, 63, 64, 66, 67, 68, 70, 71,
    73, 74, 75, 77, 78, 80, 81, 83, 85, 86, 88, 89, 91, 93, 94, 96, 98, 100,
];

pub fn encode_avif(
    image: &DynamicImage,
    _info: &ImageInfo,
    _heuristics: HeuristicsInfo,
    quality: u8,
) -> ImageResult<EncodedImage> {
    use image::codecs::avif::{AvifEncoder, ColorSpace};

    debug_assert!(quality <= 100);

    let mut buffer = Vec::new();

    let (width, height) = image.dimensions();

    AvifEncoder::new_with_speed_quality(
        &mut buffer,
        if quality < 90 { 5 } else { 4 },
        JPEG_TO_AVIF_QUALITY[quality as usize],
    )
    .with_colorspace(ColorSpace::Bt709)
    .write_image(image.as_bytes(), width, height, image.color())?;

    Ok(EncodedImage {
        buffer,
        width,
        height,
        format: ImageFormat::Avif,
    })
}

#[cfg(test)]
mod tests {
    use super::map_jpeg_to_avif_quality;

    #[test]
    fn test_map_jpeg_to_avif_quality() {
        assert_eq!(map_jpeg_to_avif_quality(0), 0);
        assert_eq!(map_jpeg_to_avif_quality(100), 100);

        assert_eq!(map_jpeg_to_avif_quality(50), 34);

        for q in 0..100 {
            print!("{},", map_jpeg_to_avif_quality(q));
        }
    }
}
