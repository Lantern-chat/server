use smol_str::SmolStr;

pub fn encode(mut x: u64) -> SmolStr {
    if x == 0 {
        return SmolStr::new_inline("0");
    }

    let mut buf = [0u8; 11];
    let mut i = 0;

    const CHARSET: &'static [u8] = b"0123456789abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ";

    while x != 0 {
        buf[i] = CHARSET[(x % 62) as usize];
        x /= 62;
        i += 1;
    }

    SmolStr::new_inline(unsafe { std::str::from_utf8_unchecked(&buf[..i]) })
}

pub fn decode(s: &str) -> u64 {
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
