use image::{ColorType, DynamicImage, GenericImageView, ImageFormat, ImageResult};

use crate::read_image::ImageInfo;

use super::EncodedImage;

#[allow(unused)]
pub fn encode_jpeg(image: &DynamicImage, info: &ImageInfo, quality: u8) -> ImageResult<EncodedImage> {
    debug_assert!(quality <= 100);

    let (width, height) = image.dimensions();

    let mut out = Vec::new();

    //if false {
    //    use image::codecs::jpeg::JpegEncoder;
    //
    //    JpegEncoder::new_with_quality(&mut out, 99).encode_image(image)?;
    //}

    // if false {
    //     use jpeg_encoder::{ColorType as C, Encoder, QuantizationTableType as Q, SamplingFactor as S};
    //
    //     let mut encoder = Encoder::new(&mut out, 90);
    //     encoder.set_optimized_huffman_tables(true);
    //     encoder.set_quantization_tables(Q::VisualDetectionModel, Q::VisualDetectionModel);
    //     //encoder.set_sampling_factor(S::F_1_1);
    //
    //     encoder
    //         .encode(
    //             image.as_bytes(),
    //             width as u16,
    //             height as u16,
    //             match image.color() {
    //                 ColorType::Rgb8 => C::Rgb,
    //                 ColorType::L8 => C::Luma,
    //                 _ => unreachable!(),
    //             },
    //         )
    //         .unwrap();
    // }

    if true {
        use mozjpeg::{qtable, ColorSpace, Compress};

        let mut encoder = Compress::new(match image.color() {
            ColorType::Rgb8 => ColorSpace::JCS_RGB,
            ColorType::Rgba8 => ColorSpace::JCS_EXT_RGBA,
            _ => unreachable!(),
        });

        encoder.set_size(width as usize, height as usize);
        encoder.set_quality(quality as f32);

        // with decreasing quality, increase smoothing, from 5% at 100 to 40% at 0
        encoder.set_smoothing_factor({ 40u16.saturating_sub(quality as u16 * 7 / 20) } as u8);

        //encoder.set_chroma_qtable(&qtable::AnnexK_Chroma);
        //encoder.set_luma_qtable(&qtable::AnnexK_Luma);

        let mut components = encoder.components_mut();
        for component in components {
            component.h_samp_factor = 1;
            component.v_samp_factor = 1;
        }

        encoder.set_mem_dest();
        encoder.start_compress();
        encoder.write_scanlines(image.as_bytes());
        encoder.finish_compress();

        out = encoder.data_to_vec().unwrap();
    }

    log::trace!("JPEG Encoder produced {} bytes", out.len());

    Ok(EncodedImage {
        buffer: out,
        width,
        height,
        format: ImageFormat::Jpeg,
    })
}
