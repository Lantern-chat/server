use std::f32::consts::PI;
use std::io;

use byteorder::BigEndian;
use byteorder::WriteBytesExt;

use crate::common::{linear_to_srgb, roundup4, srgb_to_linear};

#[inline(always)]
fn multiply_basis_function(
    xc: usize,
    yc: usize,
    w: usize,
    h: usize,
    rgb: &[u8],
    channels: usize,
) -> [f32; 3] {
    let mut r = 0.0;
    let mut g = 0.0;
    let mut b = 0.0;

    let nx = PI * (xc as f32) / (w as f32);
    let ny = PI * (yc as f32) / (h as f32);

    for y in 0..h {
        for x in 0..w {
            let basis = (x as f32 * nx).cos() * (y as f32 * ny).cos();

            let i = channels * (x + y * w);

            r += basis * srgb_to_linear(rgb[i + 0]);
            g += basis * srgb_to_linear(rgb[i + 1]);
            b += basis * srgb_to_linear(rgb[i + 2]);
        }
    }

    let n = 1.0; //if xc == 0 && yc == 0 { 1.0 } else { 2.0 };
    let scale = n / (w as f32 * h as f32);

    [r * scale, g * scale, b * scale]
}

fn encode_dc([r, g, b]: [f32; 3]) -> u32 {
    let r = linear_to_srgb(r) as u32;
    let g = linear_to_srgb(g) as u32;
    let b = linear_to_srgb(b) as u32;

    (r << 16) | (g << 8) | b
}

fn sign_sqrt(x: f32) -> f32 {
    x.abs().sqrt().copysign(x)
}

fn encode_ac([r, g, b]: [f32; 3], m: f32) -> u16 {
    const A: f32 = 9.0;
    const C: f32 = 18.0;

    let im = 1.0 / m;
    let r = (sign_sqrt(r * im) * A + A).round().min(C) as u16;
    let g = (sign_sqrt(g * im) * A + A).round().min(C) as u16;
    let b = (sign_sqrt(b * im) * A + A).round().min(C) as u16;

    r * 19 * 19 + g * 19 + b
}

pub fn encode(
    xc: usize,
    yc: usize,
    w: usize,
    h: usize,
    rgb: &mut [u8],
    channels: usize,
) -> io::Result<Vec<u8>> {
    if channels == 4 {
        // premultiply alpha
        for y in 0..h {
            for x in 0..w {
                let i = 4 * (x + y * w);

                match rgb[i + 3] {
                    255 => continue,
                    0 => rgb[i..(i + 3)].fill(0),
                    alpha => {
                        let a = alpha as f32 / 255.0;

                        rgb[i..(i + 3)].iter_mut().for_each(|c| {
                            //*c = (*c as f32 * a) as u8;

                            *c = linear_to_srgb(srgb_to_linear(*c) * a);
                        });
                    }
                }
            }
        }
    } else {
        assert_eq!(channels, 3);
    }

    let buf_size = roundup4(4 + xc * yc * 2);
    let mut buf = Vec::with_capacity(buf_size);

    let mut factors: [[[f32; 3]; 9]; 9] = [[[0.0; 3]; 9]; 9];

    for (y, y_factors) in factors.iter_mut().enumerate().take(yc) {
        for (x, factor) in y_factors.iter_mut().enumerate().take(xc) {
            *factor = multiply_basis_function(x, y, w, h, rgb, channels);
        }
    }

    let size_flag = (yc - 1) * 9 + (xc - 1);
    buf.write_u8(size_flag as u8)?;

    let max_value;

    let ac_count = xc * yc - 1;

    if ac_count > 0 {
        let mut actual_max: f32 = 0.0;
        for (y, y_factors) in factors.iter().enumerate().take(yc) {
            for (x, factor) in y_factors.iter().enumerate().take(xc) {
                if y == 0 && x == 0 {
                    continue;
                }
                let [r, g, b] = *factor;
                actual_max = actual_max.max(r).max(g).max(b);
            }
        }

        println!("Actual max: {}", actual_max);

        let q_max_value = ((actual_max * 166.0 - 0.5) as u8).min(82);
        max_value = (q_max_value as f32 + 1.0) / 166.0;
        buf.write_u8(q_max_value)?;
    } else {
        max_value = 1.0;
        buf.write_i8(0)?;
    };

    buf.write_u32::<BigEndian>(encode_dc(factors[0][0]))?;

    for (y, y_factors) in factors.iter().enumerate().take(yc) {
        for (x, factor) in y_factors.iter().enumerate().take(xc) {
            if y == 0 && x == 0 {
                continue;
            }
            buf.write_u16::<BigEndian>(encode_ac(*factor, max_value))?;
        }
    }

    buf.resize(buf_size, 0);

    Ok(buf)
}

pub fn num_components(width: u32, height: u32) -> (usize, usize) {
    let ratio = width as f32 / height as f32;

    let mut cx = 9;
    let mut cy = 9;

    if ratio < 1.0 {
        cx = (9.0 * ratio).round() as usize;
    } else {
        cy = (9.0 / ratio).round() as usize;
    }

    (cx, cy)
}
