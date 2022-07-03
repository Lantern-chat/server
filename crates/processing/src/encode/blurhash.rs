use image::{DynamicImage, GenericImageView};

pub fn gen_blurhash(image: &DynamicImage) -> Option<Vec<u8>> {
    let has_alpha = image.color().has_alpha();
    let (width, height) = image.dimensions();

    let (xc, yc) = blurhash::encode::num_components(width, height);

    let hash = blurhash::encode::encode::<true>(
        xc,
        yc,
        width as usize,
        height as usize,
        image.as_bytes(),
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
