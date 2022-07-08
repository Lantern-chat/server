use image::{ColorType, DynamicImage, GenericImageView};
use std::io::{self, Write};

use crate::read_image::ImageInfo;

// TODO: Work on color space parts
#[allow(unused_imports, unused_variables)]
pub fn encode_png<W: Write>(mut w: W, image: &DynamicImage, info: &ImageInfo, quality: u8) -> io::Result<()> {
    use png::{
        AdaptiveFilterType, BitDepth, Compression, Encoder as PngEncoder, FilterType, ScaledFloat,
        SourceChromaticities, SrgbRenderingIntent,
    };

    debug_assert!(quality <= 100);

    let bytes = image.as_bytes();
    let (width, height) = image.dimensions();
    let color = image.color();
    let num_pixels = width as usize * height as usize;

    // 1.5 bytes per pixel
    const BYTES_PER_PIXEL_D: usize = 3;
    const BYTES_PER_PIXEL_N: usize = 2;

    let expected_bytes = (num_pixels * BYTES_PER_PIXEL_D) / BYTES_PER_PIXEL_N;

    let mut out = Vec::with_capacity(1024 * 4);

    let mut encoder = PngEncoder::new(&mut out, width, height);

    encoder.set_depth(BitDepth::Eight);
    encoder.set_color(match color {
        ColorType::Rgb8 => png::ColorType::Rgb,
        ColorType::Rgba8 => png::ColorType::Rgba,
        _ => unreachable!(),
    });

    //encoder.set_trns(&[0xFFu8, 0xFFu8, 0xFFu8, 0xFFu8] as &'static [u8]);
    encoder.set_compression(Compression::Fast);
    encoder.set_filter(FilterType::NoFilter);
    encoder.set_adaptive_filter(AdaptiveFilterType::NonAdaptive);

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

    log::trace!("Initial PNG Encoded, now optimizing...");

    let small = num_pixels <= (256 * 256);
    let mut opts = oxipng::Options::from_preset(if small { 3 } else { 1 });
    opts.palette_reduction = true;
    opts.bit_depth_reduction = quality <= 60;

    let out = match oxipng::optimize_from_memory(&out, &opts) {
        Ok(new_out) => {
            log::trace!("PNG optimized from {} to {} bytes", out.len(), new_out.len());
            new_out
        }
        Err(e) => {
            log::error!("Error optimizing PNG: {}", e);
            out
        }
    };

    log::trace!(
        "PNG Encoder expected {expected_bytes} bytes, got {} bytes",
        out.len()
    );

    Ok({
        w.write_all(&out)?;
    })
}
