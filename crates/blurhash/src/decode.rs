use std::f32::consts::PI;
use std::io;

use byteorder::BigEndian;
use byteorder::ReadBytesExt;

use crate::common::{linear_to_srgb, srgb_to_linear};

fn decode_dc(value: u32) -> [f32; 3] {
    let r = value >> 16;
    let g = value >> 8;
    let b = value;

    [
        srgb_to_linear(r as u8),
        srgb_to_linear(g as u8),
        srgb_to_linear(b as u8),
    ]
}

fn decode_ac(value: u16, max: f32) -> [f32; 3] {
    const A: f32 = 9.0;

    let r = ((value / (19 * 19)) as f32 - A) / A;
    let g = (((value / 19) % 19) as f32 - A) / A;
    let b = ((value % 19) as f32 - A) / A;

    fn sign_sqr(x: f32) -> f32 {
        (x * x).copysign(x)
    }

    [sign_sqr(r) * max, sign_sqr(g) * max, sign_sqr(b) * max]
}

pub fn is_valid(mut hash: &[u8]) -> io::Result<bool> {
    let hash_len = hash.len();
    if hash_len < 6 {
        return Ok(false);
    }

    let size_flag = hash.read_u8()?;
    let y = (size_flag as usize / 9) + 1;
    let x = (size_flag as usize % 9) + 1;

    let expected = 4 + 2 * x * y;

    Ok(expected == hash_len)
}

pub fn decode(mut hash: &[u8], w: usize, h: usize, punch: f32) -> io::Result<Vec<u8>> {
    let hash_len = hash.len();
    if hash_len < 6 {
        return Ok(Vec::new());
    }

    let size_flag = hash.read_u8()? as usize;
    let cy = (size_flag / 9) + 1;
    let cx = (size_flag % 9) + 1;

    let num_colors = cx * cy;

    let q_max_value = hash.read_u8()?;
    let max_value = (q_max_value + 1) as f32 / 166.0;

    let mut colors = vec![[0.0f32; 3]; num_colors];

    colors[0] = decode_dc(hash.read_u32::<BigEndian>()?);

    let mc = max_value * punch;
    for i in 1..num_colors {
        colors[i] = decode_ac(hash.read_u16::<BigEndian>()?, mc);
    }

    let mut out = vec![0; w * h * 3];

    let iw = PI / w as f32;
    let ih = PI / h as f32;

    for y in 0..h {
        for x in 0..w {
            let mut r = 0.0;
            let mut g = 0.0;
            let mut b = 0.0;

            let xf = x as f32 * iw;
            let yf = y as f32 * ih;

            for j in 0..cy {
                for i in 0..cx {
                    let basis = (xf * i as f32).cos() * (yf * j as f32).cos();
                    let idx = i + j * cx;
                    r += colors[idx][0] * basis;
                    g += colors[idx][1] * basis;
                    b += colors[idx][2] * basis;
                }
            }

            let idx = 3 * (x + y * w);

            out[idx + 0] = linear_to_srgb(r);
            out[idx + 1] = linear_to_srgb(g);
            out[idx + 2] = linear_to_srgb(b);
        }
    }

    Ok(out)
}
