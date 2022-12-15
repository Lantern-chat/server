use std::marker::PhantomData;

use image::{math::Rect, DynamicImage, GenericImageView, ImageBuffer, Pixel, RgbaImage};

fn sinc(mut a: f32) -> f32 {
    a *= std::f32::consts::PI;
    a.sin() / a
}

pub fn lanczos(x: f32, t: f32) -> f32 {
    if x.abs() < t {
        if x != 0.0 {
            sinc(x) * sinc(x / t)
        } else {
            1.0
        }
    } else {
        0.0
    }
}

pub fn crop_and_resize<I, P>(
    image: &I,
    rect: Rect,
    new_width: u32,
    new_height: u32,
) -> ImageBuffer<P, Vec<u8>>
where
    I: GenericImageView<Pixel = P>,
    P: Pixel<Subpixel = u8>,
{
    let view = image.view(rect.x, rect.y, rect.width, rect.height);
    resize(&*view, new_width, new_height)
}

/// Based on image::imageops::resize routines, but merged together and only using a single
/// line buffer to reduce memory usage by a factor of `new_height`
pub fn resize<I, P>(image: &I, new_width: u32, new_height: u32) -> ImageBuffer<P, Vec<u8>>
where
    I: GenericImageView<Pixel = P>,
    P: Pixel<Subpixel = u8>,
{
    let (width, height) = image.dimensions();

    let new_width = new_width.min(width);
    let new_height = new_height.min(height);

    // TODO: Faster routines for resizing only a single dimension
    // if width == new_width {
    //     return resize_vertical(image, new_height);
    // } else if height == new_height {
    //     return resize_horizontal(image, new_width);
    // }

    let h1 = height as u64 - 1;
    let w1 = width as u64 - 1;

    let w_ratio = width as f32 / new_width as f32;
    let h_ratio = height as f32 / new_height as f32;

    let w_sratio = if w_ratio < 1.0 { 1.0 } else { w_ratio };
    let h_sratio = if h_ratio < 1.0 { 1.0 } else { h_ratio };

    let w_isratio = 1.0 / w_sratio;
    let h_isratio = 1.0 / h_sratio;

    let support = 2.5;
    let w_src_support = support * w_sratio;
    let h_src_support = support * h_sratio;

    let num_channels = P::CHANNEL_COUNT as usize;
    let mut line_buffer = vec![0.0f32; num_channels * width as usize];
    let mut ws: Vec<f32> = Vec::new();

    let mut out: ImageBuffer<P, Vec<u8>> = ImageBuffer::new(new_width, new_height);

    // for every vertical line
    for outy in 0..new_height {
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

        ws.clear();
        let mut sum = 0.0;
        for i in top..bottom {
            let w = lanczos((i as f32 - inputy) * h_isratio, support);
            ws.push(w);
            sum += w;
        }

        // normalize and add u8->f32 factor
        let factor = (1.0 / 255.0) / sum;
        ws.iter_mut().for_each(|w| *w *= factor);

        let mut offset = 0;
        for x in 0..width {
            let t = &mut [0.0f32; 4][..num_channels];

            for (i, &w) in ws.iter().enumerate() {
                let p = image.get_pixel(x, top + i as u32);

                for (t, &c) in t.iter_mut().zip(p.channels()) {
                    *t += c as f32 * w;
                }
            }

            // insert vertical resampling into line buffer
            let next_offset = offset + num_channels;
            line_buffer[offset..next_offset].copy_from_slice(t);
            offset = next_offset;
        }

        for outx in 0..new_width {
            let inputx = (outx as f32 + 0.5) * w_ratio;

            let left = (inputx - w_src_support) as u64; // truncate f32 -> u64
            let left = left.min(w1);

            let right = (inputx + w_src_support) as u64;
            let right = right.clamp(left + 1, width as u64);

            let left = left as u32;
            let right = right as u32;

            // Go back to left boundary of pixel, to properly compare with i
            // below, as the kernel treats the centre of a pixel as 0.
            let inputx = inputx - 0.5;

            let t = &mut [0.0f32; 4][..num_channels];

            let mut sum = 0.0;
            let mut offset = left as usize * num_channels;
            for i in left..right {
                let w = lanczos((i as f32 - inputx) * w_isratio, support);
                sum += w;

                let next_offset = offset + num_channels;
                for (t, &c) in t.iter_mut().zip(&line_buffer[offset..next_offset]) {
                    *t += w * c;
                }

                offset = next_offset;
            }

            // normalize and add f32->u8 factor
            let factor = 255.0 / sum;
            for (&t, c) in t.iter().zip(out.get_pixel_mut(outx, outy).channels_mut()) {
                // f32->u8 automatically clamps
                *c = (t * factor) as u8;
            }
        }
    }

    out
}

// fn resize_vertical<I, P>(image: &I, new_height: u32) -> ImageBuffer<P, Vec<u8>>
// where
//     I: GenericImageView<Pixel = P>,
//     P: Pixel<Subpixel = u8>,
// {
//     todo!()
// }

// fn resize_horizontal<I, P>(image: &I, new_width: u32) -> ImageBuffer<P, Vec<u8>>
// where
//     I: GenericImageView<Pixel = P>,
//     P: Pixel<Subpixel = u8>,
// {
//     todo!()
// }

pub trait ReduceSubpixel {
    fn to_u8(self) -> u8;
}

impl ReduceSubpixel for u8 {
    #[inline(always)]
    fn to_u8(self) -> u8 {
        self
    }
}

impl ReduceSubpixel for u16 {
    #[inline(always)]
    fn to_u8(self) -> u8 {
        (self >> 8) as u8
    }
}

impl ReduceSubpixel for f32 {
    #[inline(always)]
    fn to_u8(self) -> u8 {
        (self * 255.0) as u8
    }
}

pub struct ReducedView<'a, S, P> {
    inner: &'a S,
    _pixel: PhantomData<P>,
}

impl<'a, S, P> ReducedView<'a, S, P> {
    pub fn new(inner: &'a S) -> Self {
        ReducedView {
            inner,
            _pixel: PhantomData,
        }
    }
}

pub type GenericImageViewPixel<S> = <S as GenericImageView>::Pixel;
pub type GenericImageViewSubpixel<S> = <GenericImageViewPixel<S> as Pixel>::Subpixel;

fn integer_luma(r: u8, g: u8, b: u8) -> u8 {
    // 255 * 72(max channel) * 3 <= 2^16-1
    let r = r as u16 * 21;
    let g = g as u16 * 72;
    let b = b as u16 * 7;

    ((r + g + b) / 100).min(255) as u8
}

#[inline(always)]
pub fn reduce_pixel<FROM: Pixel, TO: Pixel>(mut channels: [u8; 4]) -> [u8; 4] {
    match (FROM::CHANNEL_COUNT, TO::CHANNEL_COUNT) {
        // L -> RGB
        (1, 3) => {
            channels[1] = channels[0];
            channels[2] = channels[0];
        }
        // L -> RGBA
        (1, 4) => {
            channels[1] = channels[0];
            channels[2] = channels[0];
            channels[3] = 255;
        }
        // LA -> RGBA
        (2, 4) => {
            // move alpha
            channels[3] = channels[1];
            // L->RGB
            channels[1] = channels[0];
            channels[2] = channels[0];
        }
        // RGB -> RGBA (give full opacity)
        (3, 4) => {
            channels[3] = 255;
        }
        // RGBA -> LA
        (4, 2) => {
            channels[0] = integer_luma(channels[0], channels[1], channels[2]);

            // move alpha
            channels[1] = channels[3];
        }
        // RGB -> LA
        (3, 2) => {
            channels[0] = integer_luma(channels[0], channels[1], channels[2]);
            channels[1] = 255;
        }
        // RGBA -> L
        (3 | 4, 1) => {
            channels[0] = integer_luma(channels[0], channels[1], channels[2]);
        }
        _ => {}
    }

    channels
}

impl<S, P> GenericImageView for ReducedView<'_, S, P>
where
    S: GenericImageView,
    GenericImageViewSubpixel<S>: ReduceSubpixel,
    P: Pixel<Subpixel = u8>,
{
    type Pixel = P;

    #[inline]
    fn dimensions(&self) -> (u32, u32) {
        self.inner.dimensions()
    }

    #[inline]
    fn bounds(&self) -> (u32, u32, u32, u32) {
        self.inner.bounds()
    }

    #[inline]
    fn get_pixel(&self, x: u32, y: u32) -> Self::Pixel {
        let mut channels = [0u8; 4];
        for (dst, src) in channels.iter_mut().zip(self.inner.get_pixel(x, y).channels()) {
            *dst = src.to_u8();
        }

        channels = reduce_pixel::<S::Pixel, P>(channels);

        *P::from_slice(&channels[..P::CHANNEL_COUNT as usize])
    }
}

pub fn reduce_to_u8<I, FP, P>(image: &I) -> ImageBuffer<P, Vec<u8>>
where
    I: GenericImageView<Pixel = FP>,
    FP: Pixel,
    <FP as Pixel>::Subpixel: ReduceSubpixel,
    P: Pixel<Subpixel = u8>,
{
    let (width, height) = image.dimensions();
    let mut out: ImageBuffer<P, Vec<u8>> = ImageBuffer::new(width, height);
    let view = ReducedView::<_, P>::new(image);

    for (dst, (_, _, src)) in out.pixels_mut().zip(view.pixels()) {
        for (dc, &sc) in dst.channels_mut().iter_mut().zip(src.channels()) {
            *dc = sc;
        }
    }

    out
}

pub fn crop_and_reduce(image: &DynamicImage, Rect { x, y, width, height }: Rect) -> DynamicImage {
    match image {
        DynamicImage::ImageRgb8(_) | DynamicImage::ImageRgba8(_) => image.crop_imm(x, y, width, height),
        DynamicImage::ImageLuma8(img) => {
            DynamicImage::ImageRgb8(reduce_to_u8(&*img.view(x, y, width, height)))
        }
        DynamicImage::ImageLumaA8(img) => {
            DynamicImage::ImageRgba8(reduce_to_u8(&*img.view(x, y, width, height)))
        }
        DynamicImage::ImageLuma16(img) => {
            DynamicImage::ImageRgb8(reduce_to_u8(&*img.view(x, y, width, height)))
        }
        DynamicImage::ImageLumaA16(img) => {
            DynamicImage::ImageRgba8(reduce_to_u8(&*img.view(x, y, width, height)))
        }
        DynamicImage::ImageRgb16(img) => {
            DynamicImage::ImageRgb8(reduce_to_u8(&*img.view(x, y, width, height)))
        }
        DynamicImage::ImageRgba16(img) => {
            DynamicImage::ImageRgba8(reduce_to_u8(&*img.view(x, y, width, height)))
        }
        DynamicImage::ImageRgb32F(img) => {
            DynamicImage::ImageRgb8(reduce_to_u8(&*img.view(x, y, width, height)))
        }
        DynamicImage::ImageRgba32F(img) => {
            DynamicImage::ImageRgba8(reduce_to_u8(&*img.view(x, y, width, height)))
        }

        // DynamicImage is non-exhaustive, so fallback to two-step crop and convert
        _ => {
            let image = image.crop_imm(x, y, width, height);

            match image.color().has_alpha() {
                true => DynamicImage::ImageRgba8(image.to_rgba8()),
                false => DynamicImage::ImageRgb8(image.to_rgb8()),
            }
        }
    }
}

pub fn crop_and_reduce_and_resize(
    image: &DynamicImage,
    Rect { x, y, width, height }: Rect,
    new_width: u32,
    new_height: u32,
) -> DynamicImage {
    match image {
        DynamicImage::ImageLuma8(image) => DynamicImage::ImageRgb8(resize(
            &ReducedView::new(&*image.view(x, y, width, height)),
            new_width,
            new_height,
        )),
        DynamicImage::ImageLumaA8(image) => DynamicImage::ImageRgba8(resize(
            &ReducedView::new(&*image.view(x, y, width, height)),
            new_width,
            new_height,
        )),
        DynamicImage::ImageRgb8(image) => {
            DynamicImage::ImageRgb8(resize(&*image.view(x, y, width, height), new_width, new_height))
        }
        DynamicImage::ImageRgba8(image) => {
            DynamicImage::ImageRgba8(resize(&*image.view(x, y, width, height), new_width, new_height))
        }

        DynamicImage::ImageLuma16(img) => DynamicImage::ImageRgb8(resize(
            &ReducedView::new(&*img.view(x, y, width, height)),
            new_width,
            new_height,
        )),
        DynamicImage::ImageLumaA16(img) => DynamicImage::ImageRgba8(resize(
            &ReducedView::new(&*img.view(x, y, width, height)),
            new_width,
            new_height,
        )),
        DynamicImage::ImageRgb16(img) => DynamicImage::ImageRgb8(resize(
            &ReducedView::new(&*img.view(x, y, width, height)),
            new_width,
            new_height,
        )),
        DynamicImage::ImageRgba16(img) => DynamicImage::ImageRgba8(resize(
            &ReducedView::new(&*img.view(x, y, width, height)),
            new_width,
            new_height,
        )),
        DynamicImage::ImageRgb32F(img) => DynamicImage::ImageRgb8(resize(
            &ReducedView::new(&*img.view(x, y, width, height)),
            new_width,
            new_height,
        )),
        DynamicImage::ImageRgba32F(img) => DynamicImage::ImageRgba8(resize(
            &ReducedView::new(&*img.view(x, y, width, height)),
            new_width,
            new_height,
        )),

        // DynamicImage is non-exhaustive, so fallback to two-step crop and convert
        _ => {
            let image = image.crop_imm(x, y, width, height);

            match image.color().has_alpha() {
                true => DynamicImage::ImageRgba8(image.to_rgba8()),
                false => DynamicImage::ImageRgb8(image.to_rgb8()),
            }
        }
    }
}

pub fn fast_premultiply_alpha(image: &mut RgbaImage) {
    for p in image.pixels_mut() {
        let alpha = p.0[3] as u16;
        for c in &mut p.0[0..3] {
            *c = ((*c as u16 * alpha) >> 8) as u8;
        }
        p.0[3] = 255;
    }
}
