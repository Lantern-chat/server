#![allow(clippy::identity_op)]

use image::{DynamicImage, ImageBuffer, ImageDecoder, ImageFormat};
use png::{PixelDimensions, ScaledFloat, SourceChromaticities, SrgbRenderingIntent};

pub struct ImageInfo {
    pub width: u32,
    pub height: u32,
    pub source_gamma: Option<ScaledFloat>,
    pub source_chromaticities: Option<SourceChromaticities>,
    pub icc_profile: Option<Vec<u8>>,
    pub srgb: Option<SrgbRenderingIntent>,
    pub pixel_dims: Option<PixelDimensions>,
}

pub struct Image {
    pub image: DynamicImage,
    pub info: ImageInfo,
}

use std::io::{self, Read};

use crate::ProcessConfig;

#[derive(Debug, thiserror::Error)]
pub enum ImageReadError {
    #[error("Io Error: {0}")]
    Io(#[from] io::Error),

    #[error("Image Error: {0}")]
    Image(#[from] image::ImageError),

    #[error("Invalid Image Format")]
    InvalidImageFormat,

    #[error("Image Too Large")]
    ImageTooLarge,

    #[error("File Too Large")]
    FileTooLarge,

    #[error("Png Decode Error: {0}")]
    PngDecodeError(#[from] png::DecodingError),

    #[error("Jpeg Decode Error: {0}")]
    JpegDecodeError(#[from] jpeg_decoder::Error),

    #[error("Unsupported format")]
    Unsupported,
}

pub fn read_image<R: Read>(
    unbuffered_source: R,
    config: &ProcessConfig,
    length_hint: Option<u64>,
) -> Result<Image, ImageReadError> {
    let mut source = unbuffered_source;
    let mut any_magic_bytes = Vec::with_capacity(32);

    let format = {
        (&mut source).take(32).read_to_end(&mut any_magic_bytes)?;

        match image::guess_format(&any_magic_bytes) {
            Ok(format) => format,
            Err(_) => return Err(ImageReadError::InvalidImageFormat),
        }
    };

    // re-use format bytes, + bufreader to amatorize the chain logic + speed up decoding
    let source = io::BufReader::new(any_magic_bytes.chain(source));

    match format {
        ImageFormat::Png => read_png(source, config),
        ImageFormat::Jpeg => read_jpeg(source, config),
        ImageFormat::Gif => read_generic(image::codecs::gif::GifDecoder::new(source)?, config),
        ImageFormat::WebP => read_generic(image::codecs::webp::WebPDecoder::new(source)?, config),
        ImageFormat::Pnm => read_generic(image::codecs::pnm::PnmDecoder::new(source)?, config),
        _ => {
            // TODO: Pass this value as a config?
            let max_file_size: usize = 10 * 1024 * 1024;

            match length_hint {
                Some(length) if length >= max_file_size as u64 => {
                    return Err(ImageReadError::FileTooLarge);
                }
                _ => {}
            }

            // These formats REQUIRE Read+Seek, so buffer 10MiB or so and hope that's enough

            let mut buffer = Vec::new();

            if max_file_size >= source.take(max_file_size as u64).read_to_end(&mut buffer)? {
                return Err(ImageReadError::FileTooLarge);
            }

            let source = io::Cursor::new(buffer);

            match format {
                ImageFormat::Tiff => read_generic(image::codecs::tiff::TiffDecoder::new(source)?, config),
                ImageFormat::Tga => read_generic(image::codecs::tga::TgaDecoder::new(source)?, config),
                ImageFormat::Bmp => read_generic(image::codecs::bmp::BmpDecoder::new(source)?, config),
                ImageFormat::Ico => read_generic(image::codecs::ico::IcoDecoder::new(source)?, config),
                _ => Err(ImageReadError::InvalidImageFormat),
            }
        }
        //ImageFormat::Dds => read_generic(image::codecs::dds::DdsDecoder::new(source)?, config),
        //ImageFormat::Hdr => read_generic(image::codecs::hdr::HdrDecoder::new(source)?, config),
        //ImageFormat::OpenExr => read_generic(image::codecs::openexr::OpenExrDecoder::new(source)?, config),
        //ImageFormat::Farbfeld => read_generic(image::codecs::farbfeld::FarbfieldDecoder::new(source)?, config),
        //ImageFormat::Avif => read_generic(image::codecs::gif::GifDecoder::new(source)?, config),
    }
}

fn read_generic<'a, D: ImageDecoder<'a>>(decoder: D, config: &ProcessConfig) -> Result<Image, ImageReadError> {
    let (width, height) = decoder.dimensions();

    if (width as u64 * height as u64) > config.max_pixels as u64 {
        return Err(ImageReadError::ImageTooLarge);
    }

    let Ok(image) = DynamicImage::from_decoder(decoder) else {
        return Err(ImageReadError::InvalidImageFormat);
    };

    Ok(Image {
        image,
        info: ImageInfo {
            width,
            height,
            source_gamma: None,
            source_chromaticities: None,
            icc_profile: None,
            srgb: None,
            pixel_dims: None,
        },
    })
}

macro_rules! from_raw {
    ($info:expr, $buf:expr) => {
        match ImageBuffer::from_raw($info.width, $info.height, $buf) {
            Some(image) => image,
            None => return Err(ImageReadError::Unsupported),
        }
    };
}

/// Reads in a PNG image, converting it to 8-bit color channels and checking limits first
///
/// TODO: Convert this to a scan-line reader that can resize the input line-by-line
fn read_png<R: Read>(source: R, config: &ProcessConfig) -> Result<Image, ImageReadError> {
    use png::{BitDepth, ColorType, Decoder, Transformations};

    let mut decoder = Decoder::new(source);
    decoder.set_transformations(Transformations::EXPAND | Transformations::STRIP_16);
    decoder.set_ignore_text_chunk(true);

    let mut reader = decoder.read_info()?;

    let mut info = {
        let info = reader.info();

        if (info.width as u64 * info.height as u64) > config.max_pixels as u64 {
            return Err(ImageReadError::ImageTooLarge);
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

    let mut buf = vec![0u8; reader.output_buffer_size()];

    let frame_info = reader.next_frame(&mut buf)?;
    info.height = frame_info.height;
    info.width = frame_info.width;

    buf.truncate(frame_info.buffer_size());

    if frame_info.bit_depth != BitDepth::Eight {
        return Err(ImageReadError::Unsupported);
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

fn read_jpeg<R: Read>(source: R, config: &ProcessConfig) -> Result<Image, ImageReadError> {
    use jpeg_decoder::{Decoder, PixelFormat};

    let mut decoder = Decoder::new(source);

    decoder.scale(config.max_width as u16, config.max_height as u16)?;

    decoder.read_info()?;

    let image_info = decoder.info().unwrap();

    if (image_info.width as u64 * image_info.height as u64) > config.max_pixels as u64 {
        return Err(ImageReadError::ImageTooLarge);
    }

    let info = ImageInfo {
        width: image_info.width as u32,
        height: image_info.height as u32,
        source_gamma: None,
        source_chromaticities: None,
        icc_profile: decoder.icc_profile(),
        srgb: None,
        pixel_dims: None,
    };

    let buf = decoder.decode()?;

    let image = match image_info.pixel_format {
        PixelFormat::L8 => DynamicImage::ImageLuma8(from_raw!(info, buf)),
        PixelFormat::L16 => DynamicImage::ImageLuma8(from_raw!(info, l16_to_l8(buf))),
        PixelFormat::RGB24 => DynamicImage::ImageRgb8(from_raw!(info, buf)),
        PixelFormat::CMYK32 => DynamicImage::ImageRgb8(from_raw!(info, cmyk_to_rgb(buf))),
    };

    Ok(Image { image, info })
}

// maps 2x->x without re-allocating, slicing off the lower bits of each u16
fn l16_to_l8(mut input: Vec<u8>) -> Vec<u8> {
    let new_len = input.len() / 2;
    for i in 0..new_len {
        input[i] = input[i * 2];
    }

    input.truncate(new_len);

    input
}

// maps 4x->3x without re-allocating
fn cmyk_to_rgb(mut input: Vec<u8>) -> Vec<u8> {
    let mut rgb_offset = 0;
    let mut cmyk_offset = 0;

    let count = input.len() / 4;

    for _ in 0..count {
        let c = 255 - (input[cmyk_offset + 0] as u16);
        let m = 255 - (input[cmyk_offset + 1] as u16);
        let y = 255 - (input[cmyk_offset + 2] as u16);
        let k = 255 - (input[cmyk_offset + 3] as u16);

        // CMYK -> RGB
        let r = (k * c) / 255;
        let g = (k * m) / 255;
        let b = (k * y) / 255;

        input[rgb_offset + 0] = r as u8;
        input[rgb_offset + 1] = g as u8;
        input[rgb_offset + 2] = b as u8;

        cmyk_offset += 4;
        rgb_offset += 3;
    }

    input.truncate(3 * count);

    input
}
