use image::{DynamicImage, GenericImageView};

pub fn gen_blurhash(mut image: DynamicImage) -> Option<Vec<u8>> {
    let color = image.color();

    if !color.has_color() {
        image = match color.has_alpha() {
            true => DynamicImage::ImageRgba8(image.to_rgba8()),
            false => DynamicImage::ImageRgb8(image.to_rgb8()),
        };
    }

    let (width, height) = image.dimensions();

    let (xc, yc) = blurhash::encode::num_components(width, height);

    let hash = blurhash::encode::encode::<true>(
        xc,
        yc,
        width as usize,
        height as usize,
        image.as_bytes(),
        if color.has_alpha() { 4 } else { 3 },
    );

    match hash {
        Err(e) => {
            log::error!("Error computing blurhash for avatar: {e}");
            None
        }
        Ok(hash) => Some(hash),
    }
}
