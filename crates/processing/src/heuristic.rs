use image::{Rgb, RgbImage};

#[derive(Debug, Clone, Copy)]
pub struct EdgeInfo {
    pub average: f64,
    pub variance: f64,
}

impl EdgeInfo {
    pub fn use_lossy(&self) -> bool {
        // NOTE: This is a guestimate
        self.average < 0.03 && self.variance < 0.008
    }
}

/// Fast 3x3 image convolution, same as image's filter3x3 but for RGB_8 only
/// and without any intermediate buffers, outputting to a callback function instead
#[inline]
fn apply_kernel<F>(image: &RgbImage, mut kernel: [f32; 9], mut cb: F)
where
    F: FnMut(u32, u32, Rgb<f32>),
{
    const TAPS: &[(isize, isize)] = &[
        (-1, -1),
        (0, -1),
        (1, -1),
        (-1, 0),
        (0, 0),
        (1, 0),
        (-1, 1),
        (0, 1),
        (1, 1),
    ];

    // apply u8 -> f32 weight here
    for k in &mut kernel {
        *k /= 255.0;
    }

    let (width, height) = image.dimensions();

    for y in 1..height - 1 {
        for x in 1..width - 1 {
            let mut t = [0.0f32; 3];

            for (&k, &(a, b)) in kernel.iter().zip(TAPS) {
                let x0 = x as isize + a;
                let y0 = y as isize + b;

                let rgb = image.get_pixel(x0 as u32, y0 as u32).0;

                for (&c, f) in rgb.iter().zip(&mut t) {
                    *f += k * c as f32;
                }
            }

            for tc in &mut t {
                *tc = tc.clamp(0.0, 1.0);
            }

            cb(x, y, Rgb(t));
        }
    }
}

pub fn compute_edge_info(image: &RgbImage) -> EdgeInfo {
    // RGB, so 3 channels.
    let num_pixels = (image.as_raw().len() / 3) as f64;
    let weight = 1.0 / num_pixels;

    let mut average = 0.0;
    let mut sumsq = 0.0;

    apply_kernel(
        image,
        // Laplacian operator with diagonals
        [-1.0, -1.0, -1.0, -1.0, 8.0, -1.0, -1.0, -1.0, -1.0],
        |_x, _y, Rgb([r, g, b])| {
            #[rustfmt::skip]
            let luma =
                0.212671 * r as f64 +
                0.715160 * g as f64 +
                0.072169 * b as f64;

            average += weight * luma;
            sumsq += luma * luma;
        },
    );

    EdgeInfo {
        average,
        // NOTE: since weight is 1/n, this may be slightly biased
        variance: (sumsq - (average * average) * num_pixels) * weight,
    }
}
