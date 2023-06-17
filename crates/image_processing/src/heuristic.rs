use image::{GenericImageView, Pixel};

#[derive(Debug, Clone, Copy)]
pub struct HeuristicsInfo {
    pub luma_avg: f64,
    pub edge_average: f64,
    pub edge_variance: f64,
}

impl HeuristicsInfo {
    // NOTE: This is a guestimate
    pub fn use_lossy(&self) -> bool {
        // try to avoid images that have many lines
        self.edge_average < 0.035 && self.edge_variance < 0.013
        // if it has very little high-freq details
        && (self.edge_average > 0.005 || self.edge_variance < 0.001)
    }
}

/// Fast 3x3 image convolution, same as image's filter3x3 but for RGB_8 only
/// and without any intermediate buffers, outputting to a callback function instead
#[inline]
fn apply_kernel<I, P, F>(image: &I, mut kernel: [f32; 9], mut cb: F)
where
    I: GenericImageView<Pixel = P>,
    P: Pixel<Subpixel = u8>,
    F: FnMut(u32, u32, [f32; 3]),
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

                let p = image.get_pixel(x0 as u32, y0 as u32);

                for (&c, f) in p.channels().iter().zip(&mut t) {
                    *f += k * c as f32;
                }
            }

            for tc in &mut t {
                *tc = tc.clamp(0.0, 1.0);
            }

            cb(x, y, t);
        }
    }
}

/// Laplacian operator with diagonals
#[rustfmt::skip]
const LAPLACIAN: [f32; 9] = [
    -1.0, -1.0, -1.0,
    -1.0,  8.0, -1.0,
    -1.0, -1.0, -1.0,
];

#[rustfmt::skip]
#[inline]
fn luma([r, g, b]: [f32; 3]) -> f32 {
    0.212671 * r +
    0.715160 * g +
    0.072169 * b
}

pub fn compute_heuristics<I, P>(image: &I) -> HeuristicsInfo
where
    I: GenericImageView<Pixel = P>,
    P: Pixel<Subpixel = u8>,
{
    let (width, height) = image.dimensions();
    let n = (width as u64 * height as u64) as f64;
    let weight = 1.0 / n;

    let mut luma_avg = 0.0;
    let mut edge_average = 0.0;
    let mut sumsq = 0.0;

    apply_kernel(image, LAPLACIAN, |x, y, edge| {
        let p = image.get_pixel(x, y).to_rgb();

        luma_avg += {
            let mut rgb = [0.0f32; 3];
            for (dst, &src) in rgb.iter_mut().zip(p.channels()) {
                *dst = src as f32 * (1.0 / 255.0);
            }
            weight * luma(rgb) as f64
        };

        let luma = luma(edge) as f64;

        edge_average += weight * luma;
        sumsq += luma * luma;
    });

    HeuristicsInfo {
        luma_avg,
        edge_average,
        // NOTE: since weight is 1/n, this may be slightly biased
        edge_variance: (sumsq - (edge_average * edge_average) * n) * weight,
    }
}
