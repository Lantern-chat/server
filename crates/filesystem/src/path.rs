use std::path::PathBuf;

use sdk::Snowflake;

#[inline]
fn outer_perfect_shuffle(mut x: u64) -> u64 {
    let t = (x ^ (x >> 16)) & 0x00000000FFFF0000u64;
    x = x ^ t ^ (t << 16);

    let t = (x ^ (x >> 8)) & 0x0000FF000000FF00u64;
    x = x ^ t ^ (t << 8);

    let t = (x ^ (x >> 4)) & 0x00F000F000F000F0u64;
    x = x ^ t ^ (t << 4);

    let t = (x ^ (x >> 2)) & 0x0C0C0C0C0C0C0C0Cu64;
    x = x ^ t ^ (t << 2);

    let t = (x ^ (x >> 1)) & 0x2222222222222222u64;
    x = x ^ t ^ (t << 1);

    x
}

pub fn id_to_path(id: Snowflake, buf: &mut PathBuf) {
    let id = id.to_u64();

    // OLD KEYS: 0xb83d72c7cb466675af2fc624c16ef67d, 0x1e1f65d8c3f9e3477a6c09a2d6b86b86
    // converted to new keys via https://play.rust-lang.org/?version=stable&mode=debug&edition=2021&gist=25d9ef963dd82a5b0f9fdba168d7a3f3
    let state = ahash::RandomState::with_seeds(
        0xCE388D4A7C1DEDD9,
        0x15709E26FCDF195D,
        0x1EC91837365B0A8B,
        0x29B54AF59AF086D9,
    );

    let hash = state.hash_one(id).to_le_bytes();

    // take upper bits and use them for directories
    // using only 32 bits for this allows for up to 2^32 directories as 256 / 256 / 256 / 256
    // and on EXT4 the number of files is like 2^31, so this is way, way more than sufficient.
    let mut encoded = [0; 4 * 2];
    hex::encode_to_slice(&hash[..4], &mut encoded).unwrap();

    buf.reserve(8 + 3); // 8 bytes for path chunks + 3 slashes
    for chunk in encoded.chunks_exact(2) {
        buf.push(unsafe { std::str::from_utf8_unchecked(chunk) });
    }
}

pub fn id_to_name(id: Snowflake, buf: &mut PathBuf) {
    let bytes = outer_perfect_shuffle(id.to_u64()).to_le_bytes();

    const SHUFFLE: [usize; 8] = [7, 4, 2, 0, 1, 6, 3, 5]; // randomly generated

    let mut shuffled = [0; 8];
    for i in 0..8 {
        shuffled[i] = bytes[SHUFFLE[i]];
    }

    let mut name_encoded = [0; 8 * 4 / 3 + 4];
    match URL_SAFE_NO_PAD.encode_slice(shuffled, &mut name_encoded) {
        Ok(len) => buf.push(unsafe { std::str::from_utf8_unchecked(&name_encoded[..len]) }),
        _ => unreachable!("Encoding file id to base64 should always succeed"),
    }
}
