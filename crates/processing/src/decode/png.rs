use std::io::Read;

use image::{math::Rect, DynamicImage, ImageBuffer, Pixel};
use png::{
    BitDepth, ColorType, Decoder, PixelDimensions, ScaledFloat, SourceChromaticities, SrgbRenderingIntent,
    Transformations,
};

use crate::{
    process::{compute_crop, imageops::lanczos},
    read_image::{Image, ImageInfo, ImageReadError},
    ProcessConfig,
};

pub fn read_png<R: Read>(source: R, config: &ProcessConfig) -> Result<Image, ImageReadError> {
    let mut decoder = Decoder::new(source);
    decoder.set_transformations(Transformations::EXPAND | Transformations::STRIP_16);
    decoder.set_ignore_text_chunk(true);

    let reader = decoder.read_info()?;

    let info = {
        let info = reader.info();

        if (info.width as u64 * info.height as u64) > config.max_pixels as u64 {
            return Err(ImageReadError::ImageTooLarge);
        }

        if info.bit_depth != BitDepth::Eight {
            return Err(ImageReadError::Unsupported);
        }

        ImageInfo {
            icc_profile: info.icc_profile.as_ref().map(|icc| icc.to_vec()),
            width: info.width,
            height: info.height,
            pixel_dims: info.pixel_dims,
            source_gamma: info.source_gamma,
            source_chromaticities: info.source_chromaticities,
            srgb: info.srgb,
        }
    };

    // TODO: Extract smaller image from interlaced file?
    if reader.info().interlaced
        || reader.info().is_animated()
        || (info.width <= config.max_width && info.height <= config.max_height)
    {
        return read_png_full(reader, info);
    }

    read_png_scanline(reader, info, config)
}

fn read_png_full<R: Read>(mut reader: png::Reader<R>, mut info: ImageInfo) -> Result<Image, ImageReadError> {
    let mut buf = vec![0u8; reader.output_buffer_size()];

    let frame_info = reader.next_frame(&mut buf)?;
    info.height = frame_info.height;
    info.width = frame_info.width;

    buf.truncate(frame_info.buffer_size());

    macro_rules! from_raw {
        ($info:expr, $buf:expr) => {
            match ImageBuffer::from_raw($info.width, $info.height, $buf) {
                Some(image) => image,
                None => return Err(ImageReadError::Unsupported),
            }
        };
    }

    let image = match frame_info.color_type {
        ColorType::Grayscale => DynamicImage::ImageLuma8(from_raw!(info, buf)),
        ColorType::GrayscaleAlpha => DynamicImage::ImageLumaA8(from_raw!(info, buf)),
        ColorType::Rgb => DynamicImage::ImageRgb8(from_raw!(info, buf)),
        ColorType::Rgba => DynamicImage::ImageRgba8(from_raw!(info, buf)),
        ColorType::Indexed => unreachable!("Indexed PNG colors should expand to RGB"),
    };

    Ok(Image { image, info })
}

use crate::process::imageops::{reduce_pixel, ReduceSubpixel};

fn read_png_scanline<R: Read>(
    reader: png::Reader<R>,
    info: ImageInfo,
    config: &ProcessConfig,
) -> Result<Image, ImageReadError> {
    let image = match reader.info().color_type {
        ColorType::Grayscale => DynamicImage::ImageLuma8(read_png_scanline_color(reader, config)?),
        ColorType::GrayscaleAlpha => DynamicImage::ImageLumaA8(read_png_scanline_color(reader, config)?),
        ColorType::Rgb => DynamicImage::ImageRgb8(read_png_scanline_color(reader, config)?),
        ColorType::Rgba => DynamicImage::ImageRgba8(read_png_scanline_color(reader, config)?),
        ColorType::Indexed => unreachable!("Indexed PNG colors should expand to RGB"),
    };

    Ok(Image { image, info })
}

fn read_png_scanline_color<R: Read, P>(
    mut reader: png::Reader<R>,
    config: &ProcessConfig,
) -> Result<ImageBuffer<P, Vec<u8>>, ImageReadError>
where
    P: Pixel<Subpixel = u8>,
{
    let png::Info {
        width: raw_width,
        height: raw_height,
        ..
    } = *reader.info();

    let (Rect { x, y, width, height }, _) = compute_crop((raw_width, raw_height), *config);
    let xp = x as usize * P::CHANNEL_COUNT as usize;

    // crop any top rows by just ignoring them
    for _ in 0..y {
        reader.next_row()?;
    }

    // the crop ensures this is the correct aspect ratio
    let mut out = ImageBuffer::new(config.max_width, config.max_height);

    // number of adjacent pixels the filter should look through
    let support = 2.5f32;

    // NOTE: The sratio values would always be equal to ratio, as this is strictly downscaling

    let w_ratio = width as f32 / out.width() as f32;
    let h_ratio = height as f32 / out.height() as f32;
    let w_src_support = support * w_ratio;
    let h_src_support = support * h_ratio;

    let w_iratio = 1.0 / w_ratio;
    let h_iratio = 1.0 / h_ratio;

    let num_channels = P::CHANNEL_COUNT as usize;

    let v_extent = h_src_support.ceil() as usize;
    // vertical range the filter reads
    let lines_height = 1 + 2 * v_extent;

    // Window of output lines that will be collapsed into the final output
    //
    // [1, 2, 3, 4] >
    // [1, 2, 3, 4] > > [1, 2, 3, 4]
    // [1, 2, 3, 4] >
    let vwidth = num_channels * out.width() as usize;
    let vheight = lines_height;
    let mut v = vec![0f32; vwidth * vheight];
    let mut ws: Vec<f32> = Vec::new();

    let h1 = height as u64 - 1;
    let w1 = width as u64 - 1;

    // for each line/row available
    for outy in 0..height {
        // read in row
        let row = match reader.next_row()? {
            // crop row
            Some(row) => &row.data()[xp..],
            None => break,
        };

        // last line of v
        let line_buffer = &mut v[vwidth * (vheight - 1)..];

        // for each corresponding OUTPUT pixel in the row
        for outx in 0..out.width() {
            let inputx = (outx as f32 + 0.5) * w_ratio;

            let left = (inputx - w_src_support) as u64;
            let left = left.min(w1);

            let right = (inputx + w_src_support) as u64;
            let right = right.clamp(left + 1, width as u64);

            let left = left as u32;
            let right = right as u32;

            let inputx = inputx - 0.5;

            let t = &mut [0.0f32; 4][..num_channels];

            // for each contributing pixel to the row
            let mut sum = 0.0;
            let mut offset = left as usize * num_channels;
            for i in left..right {
                let w = lanczos((i as f32 - inputx) * w_iratio, support);
                sum += w;

                let next_offset = offset + num_channels;
                for (t, &c) in t.iter_mut().zip(&row[offset..next_offset]) {
                    *t += w * c as f32;
                }

                offset = next_offset;
            }

            // normalize and add u8->f32 factor
            let factor = (1.0 / 255.0) / sum;

            for (&t, c) in t.iter().zip(&mut line_buffer[(outx as usize * num_channels)..]) {
                *c = t * factor;
            }
        }

        // Find the point in the input image corresponding to the centre
        // of the current pixel in the output image.
        let inputy = (outy as f32 + 0.5) * h_ratio;

        let top = (inputy - h_src_support) as u64; // truncate f32 -> u64
        let top = top.min(h1);

        let bottom = (inputy + h_src_support) as u64;
        let bottom = bottom.clamp(top + 1, height as u64);

        let top = top as u32;
        let bottom = bottom as u32;

        // Go back to top boundary of pixel, to properly compare with i
        // below, as the kernel treats the centre of a pixel as 0.
        let inputy = inputy - 0.5;

        // todo

        // move up vertical lines, overwriting the first line
        // and duplicating the last line
        v.copy_within(vwidth.., 0);
    }

    Ok(out)
}
