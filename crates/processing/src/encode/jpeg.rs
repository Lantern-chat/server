use image::{
    error::{EncodingError, ImageFormatHint},
    ColorType, DynamicImage, GenericImageView, ImageError, ImageFormat, ImageResult,
};

use crate::{heuristic::HeuristicsInfo, read_image::ImageInfo};

use super::EncodedImage;

#[allow(unused)]
pub fn encode_jpeg(
    image: &DynamicImage,
    info: &ImageInfo,
    heuristics: HeuristicsInfo,
    mut quality: u8,
) -> ImageResult<EncodedImage> {
    debug_assert!(quality <= 100);

    if !heuristics.use_lossy() {
        quality = quality.saturating_add(10).min(100);
    }

    let out = match try_encode_mozjpeg(image, quality) {
        Some(out) => out,
        None => encode_fallback(image, quality)?,
    };

    log::trace!("JPEG Encoder produced {} bytes", out.len());

    let (width, height) = image.dimensions();

    Ok(EncodedImage {
        buffer: out,
        width,
        height,
        format: ImageFormat::Jpeg,
    })
}

fn try_encode_mozjpeg(image: &DynamicImage, quality: u8) -> Option<Vec<u8>> {
    let res = std::panic::catch_unwind(|| {
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

        // with decreasing quality, increase smoothing, from 15% at 100 to 50% at 0
        encoder.set_smoothing_factor({ 50u16.saturating_sub(quality as u16 * 7 / 20) } as u8);

        encoder.set_chroma_qtable(&Q::AnnexK_Chroma);
        encoder.set_luma_qtable(&Q::AnnexK_Luma);

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

        encoder.data_to_vec().unwrap()
    });

    match res {
        Ok(res) => Some(res),
        Err(_) => {
            log::error!("Error encoding JPEG with mozjpeg");

            None
        }
    }
}

fn encode_fallback(image: &DynamicImage, quality: u8) -> ImageResult<Vec<u8>> {
    use jpeg_encoder::{ColorType as C, Encoder, QuantizationTableType as Q, SamplingFactor as S};

    let mut out = Vec::new();

    let mut encoder = Encoder::new(&mut out, quality);
    encoder.set_optimized_huffman_tables(true);
    encoder.set_quantization_tables(Q::ImageMagick, Q::ImageMagick);
    encoder.set_sampling_factor(S::F_1_1);

    let (width, height) = image.dimensions();
    encoder
        .encode(
            image.as_bytes(),
            width as u16,
            height as u16,
            match image.color() {
                ColorType::Rgb8 => C::Rgb,
                ColorType::L8 => C::Luma,
                _ => unreachable!(),
            },
        )
        .map_err(|e| {
            ImageError::Encoding(EncodingError::new(ImageFormatHint::Exact(ImageFormat::Jpeg), e))
        })?;

    Ok(out)
}
