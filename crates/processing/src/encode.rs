use image::{ColorType, DynamicImage, GenericImageView, RgbImage};
use std::io;

use crate::read_image::ImageInfo;

pub struct EncodedImage {
    pub buffer: Vec<u8>,
    pub preview: Option<Vec<u8>>,
    pub width: u32,
    pub height: u32,
}

fn gen_blurhash(image: DynamicImage) -> Option<Vec<u8>> {
    let has_alpha = image.color().has_alpha();
    let (width, height) = image.dimensions();

    let mut bytes: Vec<u8> = match image.color() {
        // reuse existing byte buffer if possible
        ColorType::Rgba8 | ColorType::Rgb8 => image.into_bytes(),
        _ => {
            if has_alpha {
                image.to_rgba8().into_raw()
            } else {
                image.to_rgb8().into_raw()
            }
        }
    };

    let (xc, yc) = blurhash::encode::num_components(width, height);

    if has_alpha {
        blurhash::encode::premultiply_alpha(width as usize, height as usize, &mut bytes);
    }

    let hash = blurhash::encode::encode(
        xc,
        yc,
        width as usize,
        height as usize,
        &bytes,
        if has_alpha { 4 } else { 3 },
    );

    match hash {
        Err(e) => {
            log::error!("Error computing blurhash for avatar: {e}");
            None
        }
        Ok(hash) => Some(hash),
    }
}

#[derive(Debug, Clone, Copy)]
pub struct EdgeInfo {
    average: f64,
    variance: f64,
}

impl EdgeInfo {
    pub fn use_lossy(&self) -> bool {
        // NOTE: This is a guestimate
        self.average < 0.03 && self.variance < 0.008
    }
}

pub fn compute_edge_info(image: &RgbImage) -> EdgeInfo {
    #[rustfmt::skip]
    let edges = image::imageops::filter3x3(
        image,
        // Laplacian operator with diagonals
        &[
            -1.0, -1.0, -1.0,
            -1.0,  8.0, -1.0,
            -1.0, -1.0, -1.0
        ]
    );

    let pixels = edges.pixels();
    let num_pixels = pixels.len() as f64;
    let weight = 1.0 / num_pixels;

    let mut average = 0.0;
    let mut sumsq = 0.0;

    for pixel in pixels {
        let [r, g, b] = pixel.0;

        #[rustfmt::skip]
        let luma =
            (0.212671 / 255.0) * r as f64 +
            (0.715160 / 255.0) * g as f64 +
            (0.072169 / 255.0) * b as f64;

        average += weight * luma;
        sumsq += luma * luma;
    }

    EdgeInfo {
        average,
        // NOTE: since weight is 1/n, this may be slightly biased
        variance: (sumsq - (average * average) * num_pixels) * weight,
    }
}

// TODO: Work on color space parts
#[allow(unused_imports, unused_variables)]
pub fn encode_png_best(
    mut image: DynamicImage,
    mut preview: Option<Vec<u8>>,
    info: ImageInfo,
) -> io::Result<EncodedImage> {
    use png::{
        AdaptiveFilterType, BitDepth, Compression, Encoder as PngEncoder, FilterType, ScaledFloat,
        SourceChromaticities, SrgbRenderingIntent,
    };

    image = match image {
        DynamicImage::ImageRgba16(_) | DynamicImage::ImageRgba32F(_) => {
            DynamicImage::ImageRgba8(image.to_rgba8())
        }
        DynamicImage::ImageRgb16(_) | DynamicImage::ImageRgb32F(_) => {
            DynamicImage::ImageRgb8(image.to_rgb8())
        }
        DynamicImage::ImageLuma16(_) => DynamicImage::ImageLuma8(image.to_luma8()),
        DynamicImage::ImageLumaA16(_) => DynamicImage::ImageLumaA8(image.to_luma_alpha8()),
        DynamicImage::ImageLuma8(_) => image,
        DynamicImage::ImageLumaA8(_) => image,
        DynamicImage::ImageRgb8(_) => image,
        DynamicImage::ImageRgba8(_) => image,
        _ => {
            log::warn!("DynamicImage is non-exhaustive, reached unknown state and falling back to RGBA");
            DynamicImage::ImageRgba8(image.to_rgba8())
        }
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
        preview = gen_blurhash(image);
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

#[allow(unused)]
pub fn encode_jpeg_best(
    mut image: DynamicImage,
    mut preview: Option<Vec<u8>>,
    info: ImageInfo,
) -> io::Result<EncodedImage> {
    use image::codecs::jpeg::JpegEncoder;

    unimplemented!()
}
