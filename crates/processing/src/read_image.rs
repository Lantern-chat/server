use image::{io::Reader, DynamicImage, ImageBuffer, ImageFormat};
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

#[derive(Debug, Clone, Copy)]
pub struct Limits {
    pub max_pixels: u32,
}

use std::io::{self, BufRead, Read, Seek};

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

    #[error("Png Decode Error: {0}")]
    PngDecodeError(#[from] png::DecodingError),

    #[error("Jpeg Decode Error: {0}")]
    JpegDecodeError(#[from] jpeg_decoder::Error),

    #[error("Unsupported format")]
    Unsupported,
}

pub fn read_image<R: BufRead + Seek>(
    mut source: R,
    format: ImageFormat,
    limits: &Limits,
) -> Result<Image, ImageReadError> {
    match format {
        ImageFormat::Png => read_png(source, limits),
        ImageFormat::Jpeg => read_jpeg(source, limits),
        _ => {
            let (width, height) = {
                let mut reader = Reader::new(&mut source);
                reader.set_format(format);

                match reader.into_dimensions() {
                    Ok(dim) => dim,
                    Err(_) => return Err(ImageReadError::InvalidImageFormat),
                }
            };

            if (width * height) > limits.max_pixels {
                return Err(ImageReadError::ImageTooLarge);
            }

            source.rewind()?;

            let mut reader = Reader::new(source);
            reader.set_format(format);

            let image = match reader.decode() {
                Ok(image) => image,
                Err(_) => return Err(ImageReadError::InvalidImageFormat),
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
    }
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
fn read_png<R: Read>(source: R, limits: &Limits) -> Result<Image, ImageReadError> {
    use png::{BitDepth, ColorType, Decoder, Transformations};

    let mut decoder = Decoder::new(source);
    decoder.set_transformations(Transformations::EXPAND | Transformations::STRIP_16);

    let mut reader = decoder.read_info()?;

    let mut info = {
        let image_info = reader.info();

        if (image_info.width as u64 * image_info.height as u64) > limits.max_pixels as u64 {
            return Err(ImageReadError::ImageTooLarge);
        }

        copy_png_info(image_info)
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

fn copy_png_info(src: &png::Info) -> ImageInfo {
    ImageInfo {
        icc_profile: src.icc_profile.as_ref().map(|icc| icc.to_vec()),
        width: src.width,
        height: src.height,
        pixel_dims: src.pixel_dims,
        source_gamma: src.source_gamma,
        source_chromaticities: src.source_chromaticities,
        srgb: src.srgb,
    }
}

fn read_jpeg<R: Read>(source: R, limits: &Limits) -> Result<Image, ImageReadError> {
    use jpeg_decoder::{Decoder, PixelFormat};

    let mut decoder = Decoder::new(source);

    decoder.read_info()?;

    let image_info = decoder.info().unwrap();

    if (image_info.width as u64 * image_info.height as u64) > limits.max_pixels as u64 {
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
        PixelFormat::L16 => DynamicImage::ImageLuma8(from_raw!(info, l16_to_l8(&buf))),
        PixelFormat::RGB24 => DynamicImage::ImageRgb8(from_raw!(info, buf)),
        PixelFormat::CMYK32 => DynamicImage::ImageRgb8(from_raw!(info, cmyk_to_rgb(&buf))),
    };

    Ok(Image { image, info })
}

fn l16_to_l8(input: &[u8]) -> Vec<u8> {
    let mut output = vec![0u8; input.len() / 2];
    for (chunk, out) in input.chunks_exact(2).zip(&mut output) {
        *out = chunk[0]; // strip lower bits by ignoring the second chunk
    }

    output
}

// fn l16_to_l16(input: &[u8]) -> Vec<u16> {
//     let mut output = vec![0u16; input.len() / 2];
//     for (chunk, out) in input.chunks_exact(2).zip(&mut output) {
//         *out = ((chunk[0] as u16) << 8) | (chunk[1] as u16)
//     }

//     output
// }

fn cmyk_to_rgb(input: &[u8]) -> Vec<u8> {
    let count = input.len() / 4;
    let mut output = vec![0; 3 * count];

    let in_pixels = input[..4 * count].chunks_exact(4);
    let out_pixels = output[..3 * count].chunks_exact_mut(3);

    for (pixel, outp) in in_pixels.zip(out_pixels) {
        let c = 255 - (pixel[0] as u16);
        let m = 255 - (pixel[1] as u16);
        let y = 255 - (pixel[2] as u16);
        let k = 255 - (pixel[3] as u16);
        // CMY -> RGB
        let r = (k * c) / 255;
        let g = (k * m) / 255;
        let b = (k * y) / 255;

        outp[0] = r as u8;
        outp[1] = g as u8;
        outp[2] = b as u8;
    }

    output
}
