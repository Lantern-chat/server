use image::{GenericImageView, ImageBuffer, Pixel};

fn sinc(mut a: f32) -> f32 {
    a *= std::f32::consts::PI;
    a.sin() / a
}

fn lanczos(x: f32, t: f32) -> f32 {
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

        let top = (inputy - h_src_support) as i64; // truncate f32 -> i64
        let top = top.clamp(0, height as i64 - 1);

        let bottom = (inputy + h_src_support) as i64;
        let bottom = bottom.clamp(top + 1, height as i64);

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
            let inputx = (outx as f32 + 0.5) * w_sratio;

            let left = (inputx - w_src_support) as i64; // truncate f32 -> i64
            let left = left.clamp(0, width as i64 - 1);

            let right = (inputx + w_src_support) as i64;
            let right = right.clamp(left + 1, width as i64);

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
                *c = (t * factor).max(0.0).min(255.0) as u8;
            }
        }
    }

    out
}
