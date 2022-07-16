#[rustfmt::skip]
pub fn linear_to_srgb(v: f32) -> u8 {
    //return (v * 255.0) as u8;

    let srgb = if v <= 0.0031308 {
        v * 12.92
    } else {
        1.055 * v.powf(1.0 / 2.4) - 0.055
    };

    (srgb * 255.0).round().clamp(0.0, 255.0) as u8
}

pub fn srgb_to_linear(v: u8) -> f32 {
    //return v as f32 / 255.0;

    let v = v as f32 / 255.0;
    if v <= 0.04045 {
        v / 12.92
    } else {
        ((v + 0.055) / 1.055).powf(2.4)
    }
}

pub fn roundup4(len: usize) -> usize {
    (len + 3) & !0x03
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_roundup4() {
        assert_eq!(roundup4(3), 4);
        assert_eq!(roundup4(4), 4);
        assert_eq!(roundup4(5), 8);
        assert_eq!(roundup4(36), 36);
    }
}
