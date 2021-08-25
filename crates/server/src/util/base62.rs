pub fn encode(mut x: u64) -> String {
    if x == 0 {
        return "0".to_owned();
    }

    let mut s = String::with_capacity(11);

    const CHARSET: &'static [u8] = b"0123456789abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ";

    while x != 0 {
        s.push(CHARSET[(x % 62) as usize] as char);
        x /= 62;
    }

    s
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
