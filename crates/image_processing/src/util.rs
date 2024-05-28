use image::{DynamicImage, GenericImageView, Pixel};

pub(crate) fn actually_has_alpha(image: &DynamicImage) -> bool {
    match image {
        DynamicImage::ImageLuma8(_) => false,
        DynamicImage::ImageRgb8(_) => false,
        DynamicImage::ImageLuma16(_) => false,
        DynamicImage::ImageRgb16(_) => false,
        DynamicImage::ImageRgb32F(_) => false,

        DynamicImage::ImageLumaA8(img) => check_pixels_for_alpha(img),
        DynamicImage::ImageRgba8(img) => check_pixels_for_alpha(img),
        DynamicImage::ImageLumaA16(img) => check_pixels_for_alpha(img),
        DynamicImage::ImageRgba16(img) => check_pixels_for_alpha(img),
        DynamicImage::ImageRgba32F(img) => check_pixels_for_alpha(img),
        _ => true,
    }
}

// NOTE: These are adjusted to ignore about 1% of alpha

pub(crate) trait AlphaValue: PartialOrd + std::fmt::Debug {
    const MAX: Self;
}

impl AlphaValue for u8 {
    const MAX: u8 = u8::MAX - (u8::MAX / 100 + 1);
}

impl AlphaValue for u16 {
    const MAX: u16 = u16::MAX - (u16::MAX / 100 + 1);
}

impl AlphaValue for f32 {
    const MAX: f32 = 0.99;
}

fn check_pixels_for_alpha<G>(image: &G) -> bool
where
    G: GenericImageView<Pixel: Pixel<Subpixel: AlphaValue>>,
{
    // use last channel as alpha
    let x = <<G as GenericImageView>::Pixel as Pixel>::CHANNEL_COUNT as usize - 1;
    for (_, _, p) in image.pixels() {
        if p.channels()[x] < AlphaValue::MAX {
            return true;
        }
    }

    false
}
