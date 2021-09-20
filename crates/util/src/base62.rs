use smol_str::SmolStr;

const CHARSET: &'static [u8] = b"0123456789abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ";

fn charset(x: usize) -> u8 {
    debug_assert!(x < CHARSET.len());

    unsafe { *CHARSET.get_unchecked(x) }
}

pub fn encode64(mut x: u64) -> SmolStr {
    let mut buf = [b'0'; 11];
    let mut i = 0;

    while x != 0 {
        buf[i] = charset((x % 62) as usize);
        x /= 62;
        i += 1;
    }

    SmolStr::new_inline(unsafe { std::str::from_utf8_unchecked(&buf) })
}

use strength_reduce::StrengthReducedU128;

//use static_init::dynamic;
//#[dynamic]
//static SIXTY_TWO: StrengthReducedU128 = StrengthReducedU128::new(62);

pub fn encode128(mut x: u128) -> SmolStr {
    const SIXTY_TWO: StrengthReducedU128 = StrengthReducedU128 {
        multiplier_hi: 5488425272918362313925396894060777604,
        multiplier_lo: 43907402183346898511403175152486220834,
        divisor: 62,
    };

    let mut buf = [b'0'; 22];
    let mut i = 0;

    while x != 0 {
        let (xd, xm) = StrengthReducedU128::div_rem(x, SIXTY_TWO);

        unsafe {
            *buf.get_unchecked_mut(i) = *CHARSET.get_unchecked(xm as usize);
        }

        x = xd;
        i += 1;
    }

    SmolStr::new_inline(unsafe { std::str::from_utf8_unchecked(&buf) })
}

pub fn decode64(s: &str) -> u64 {
    // precalculated list of 62^x powers
    const POWERS: [u64; 11] = [
        1,
        62,
        3844,
        238328,
        14776336,
        916132832,
        56800235584,
        3521614606208,
        218340105584896,
        13537086546263552,
        839299365868340224,
    ];

    let s = s.as_bytes();
    let mut x = 0;
    let mut i = 0;

    for c in s[..11].iter() {
        let y = match *c as char {
            '0'..='9' => c - 48,
            'A'..='Z' => c - 29,
            'a'..='z' => c - 87,
            _ => return 0,
        };

        x += y as u64 * POWERS[i];
        i += 1;
    }

    x
}

pub fn decode128(s: &str) -> u128 {
    // precalculated list of 62^x powers
    const POWERS: [u128; 22] = [
        1,
        62,
        3844,
        238328,
        14776336,
        916132832,
        56800235584,
        3521614606208,
        218340105584896,
        13537086546263552,
        839299365868340224,
        52036560683837093888,
        3226266762397899821056,
        200028539268669788905472,
        12401769434657526912139264,
        768909704948766668552634368,
        47672401706823533450263330816,
        2955688905823059073916326510592,
        183252712161029662582812243656704,
        11361668153983839080134359106715648,
        704423425546998022968330264616370176,
        43674252383913877424036476406214950912,
    ];

    let s = s.as_bytes();
    let mut x = 0;
    let mut i = 0;

    for c in s[..22].iter() {
        let y = match *c as char {
            '0'..='9' => c - 48,
            'A'..='Z' => c - 29,
            'a'..='z' => c - 87,
            _ => return 0,
        };

        x += y as u128 * POWERS[i];
        i += 1;
    }

    x
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_base62() {
        let input = "NfUJBUoglqu1588eP4Ulx4";

        let x = decode128(input);
        let y = encode128(x);

        assert_eq!(y, input);

        let input = 235623462346;

        let x = encode128(input);
        let y = decode128(&x);

        println!("{}", x);

        assert_eq!(y, input);
    }
}
