use std::io::Write;

use image::{
    error::{EncodingError, ImageFormatHint},
    ColorType, DynamicImage, GenericImageView, ImageError, ImageFormat, ImageResult,
};

use crate::{heuristic::HeuristicsInfo, read_image::ImageInfo};

#[allow(unused)]
pub fn encode_jpeg<W: Write>(
    mut w: W,
    image: &DynamicImage,
    info: &ImageInfo,
    heuristics: HeuristicsInfo,
    mut quality: u8,
) -> ImageResult<()> {
    debug_assert!(quality <= 100);

    if !heuristics.use_lossy() {
        quality = quality.saturating_add(10).min(100);
    }

    let mut buffer = Vec::new();

    if !try_encode_mozjpeg(&mut buffer, image, quality)? {
        drop(buffer);

        return encode_fallback(&mut w, image, quality);
    }

    w.write_all(&buffer)?;

    Ok(())
}

fn try_encode_mozjpeg<W: Write>(mut w: W, image: &DynamicImage, quality: u8) -> Result<bool, std::io::Error> {
    let res = std::panic::catch_unwind(move || {
        #[allow(unused_imports)]
        use mozjpeg::{qtable as Q, ColorSpace, Compress};

        let mut encoder = Compress::new(match image.color() {
            ColorType::Rgb8 => ColorSpace::JCS_RGB,
            ColorType::Rgba8 => ColorSpace::JCS_EXT_RGBA,
            ColorType::L8 => ColorSpace::JCS_GRAYSCALE,
            _ => unimplemented!(),
        });

        let (width, height) = image.dimensions();
        encoder.set_size(width as usize, height as usize);
        encoder.set_quality(quality as f32);
        encoder.set_use_scans_in_trellis(true);
        encoder.set_optimize_coding(true);

        // with decreasing quality, increase smoothing, from 5% at 100 to 40% at 0
        encoder.set_smoothing_factor({ 40u16.saturating_sub(quality as u16 * 7 / 20) } as u8);

        // NOTE: mozjpeg seems to give better results without explicit quanization tables
        //encoder.set_chroma_qtable(&Q::NRobidoux);
        //encoder.set_luma_qtable(&Q::NRobidoux);

        if quality >= 60 {
            for component in encoder.components_mut() {
                component.h_samp_factor = 1;
                component.v_samp_factor = 1;
            }
        }

        encoder.set_mem_dest();
        encoder.start_compress();
        assert!(encoder.write_scanlines(image.as_bytes()));
        encoder.finish_compress();

        encoder
    });

    // NOTE: Getting the output slice and dropping the encoder should be
    // safe, unless there is a double-free or something obscure that actually
    // warrants a panic

    match res {
        Ok(mut encoder) => match encoder.data_as_mut_slice() {
            Ok(buf) => w.write_all(buf).map(|_| true),
            Err(_) => Ok(false),
        },
        Err(_) => {
            log::error!("Error encoding JPEG with mozjpeg");

            Ok(false)
        }
    }
}

fn encode_fallback<W: Write>(w: W, image: &DynamicImage, quality: u8) -> ImageResult<()> {
    use jpeg_encoder::{ColorType as C, Encoder, QuantizationTableType as Q, SamplingFactor as S};

    let mut encoder = Encoder::new(w, quality);
    encoder.set_optimized_huffman_tables(true);
    encoder.set_quantization_tables(Q::ImageMagick, Q::ImageMagick);
    encoder.set_sampling_factor(S::F_1_1);

    let (width, height) = image.dimensions();

    let res = encoder.encode(
        image.as_bytes(),
        width as u16,
        height as u16,
        match image.color() {
            ColorType::Rgb8 => C::Rgb,
            ColorType::Rgba8 => C::Rgba,
            ColorType::L8 => C::Luma,
            _ => unreachable!(),
        },
    );

    res.map_err(|e| ImageError::Encoding(EncodingError::new(ImageFormatHint::Exact(ImageFormat::Jpeg), e)))
}
