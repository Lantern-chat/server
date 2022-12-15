use std::io::Write;

use image::{DynamicImage, GenericImageView, ImageFormat, ImageResult};

use crate::{heuristic::HeuristicsInfo, read_image::ImageInfo};

#[cfg(test)]
fn map_jpeg_to_avif_quality(q: u8) -> u8 {
    if q == 100 || q == 0 {
        return q;
    }

    let a = 1.0 / 40.0;
    let c = 100.0 / (2f32.powf(100.0 * a) - 1.0);

    let x = q as f32;
    let d = c.mul_add(2f32.powf(x * a), -c);

    let t = x / 100.0;

    ((1.0 - t) * x + t * d).ceil() as u8
}

/// `map_jpeg_to_avif_quality` made into a lookup table
static JPEG_TO_AVIF_QUALITY: [u8; 101] = [
    0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 13, 14, 15, 16, 17, 17, 18, 19, 20, 21, 21, 22, 23, 24, 24,
    25, 26, 27, 27, 28, 29, 30, 30, 31, 32, 32, 33, 34, 35, 35, 36, 37, 37, 38, 39, 40, 40, 41, 42, 43, 43,
    44, 45, 46, 46, 47, 48, 49, 50, 51, 51, 52, 53, 54, 55, 56, 57, 58, 59, 60, 61, 62, 63, 64, 66, 67, 68,
    69, 71, 72, 73, 75, 76, 77, 79, 81, 82, 84, 85, 87, 89, 91, 93, 94, 96, 98, 100,
];

pub fn encode_avif<W: Write>(
    mut w: W,
    image: &DynamicImage,
    _info: &ImageInfo,
    _heuristics: HeuristicsInfo,
    quality: u8,
) -> ImageResult<()> {
    use ravif::{ColorSpace, Encoder, Img};
    use rgb::AsPixels;

    debug_assert!(quality <= 100);

    let (width, height) = image.dimensions();

    let small = (width * height) <= (256 * 256);
    let speed = match (quality < 90, small) {
        (true, false) => 8,  // low-quality, large image, gotta go fast
        (true, true) => 5,   // low-quality, small image, slightly faster than default
        (false, false) => 6, // high-quality, large image, can't spend too much time on it
        (false, true) => 4,  // high-quality, small image, try to optimize well enough
    };
    let quality = JPEG_TO_AVIF_QUALITY[quality as usize];

    let encoder = Encoder::new()
        .with_quality(quality as f32)
        .with_alpha_quality(quality as f32)
        .with_speed(speed)
        .with_alpha_color_mode(ravif::AlphaColorMode::UnassociatedClean)
        .with_internal_color_space(ColorSpace::YCbCr)
        // try to save some parallelism on small images, but larger images require extra
        .with_num_threads(Some(if small { 1 } else { 3 }));

    let res = match image {
        DynamicImage::ImageRgb8(image) => encoder
            .encode_rgb(Img::new(
                image.as_raw().as_pixels(),
                width as usize,
                height as usize,
            ))
            .map(|r| r.avif_file),
        DynamicImage::ImageRgba8(image) => encoder
            .encode_rgba(Img::new(
                image.as_raw().as_pixels(),
                width as usize,
                height as usize,
            ))
            .map(|r| r.avif_file),
        _ => unimplemented!(),
    };

    // TODO: Figure out a better way to write directly to the writer?
    // the av1 serializer supports merging components into a stream,
    // but ravif chooses to put it all in a Vec

    match res {
        Ok(buffer) => Ok({
            w.write_all(&buffer)?;
        }),
        Err(err) => Err(image::ImageError::Encoding(image::error::EncodingError::new(
            ImageFormat::Avif.into(),
            err,
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::{map_jpeg_to_avif_quality, JPEG_TO_AVIF_QUALITY};

    #[test]
    fn test_map_jpeg_to_avif_quality() {
        for q in 0..101 {
            print!("{},", map_jpeg_to_avif_quality(q));
        }

        assert_eq!(map_jpeg_to_avif_quality(0), JPEG_TO_AVIF_QUALITY[0]);
        assert_eq!(map_jpeg_to_avif_quality(100), JPEG_TO_AVIF_QUALITY[100]);
        assert_eq!(map_jpeg_to_avif_quality(50), JPEG_TO_AVIF_QUALITY[50]);
    }
}
