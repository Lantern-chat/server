use std::fs::File;

use image::ImageDecoder;

use z85::{FromZ85, ToZ85};

fn main() {
    let mut file = File::open("deps/blurhash/e.jpg").unwrap();

    let mut d = image::codecs::jpeg::JpegDecoder::new(&mut file).unwrap();

    let res = d.dimensions();
    let (w, h) = d.scale(9, 9).unwrap();
    let mut bytes = vec![0; d.total_bytes() as usize];
    d.read_image(&mut bytes).unwrap();

    let ratio = w as f32 / h as f32;

    let c = 9;

    let mut cx = c;
    let mut cy = c;

    if ratio < 1.0 {
        cx = (c as f32 * ratio).round() as usize;
    } else {
        cy = (c as f32 / ratio).round() as usize;
    }

    println!("{cx}x{cy}");

    let buf = blurhash::encode::encode(cx, cy, w as usize, h as usize, &mut bytes, 3).unwrap();

    let encoded = buf.to_z85().unwrap();

    println!("{w}x{h}={}\n{encoded}", buf.len());

    let decoded = encoded.from_z85().unwrap();

    println!("{:?}", decoded);

    //let o = 512;
    let ox = res.0 as u32;
    let oy = res.1 as u32;

    let blurred = blurhash::decode::decode(&decoded, ox as usize, oy as usize, 1.4).unwrap();

    //println!("{:?}", blurred);

    let blurred_img = image::RgbImage::from_raw(ox, oy, blurred).unwrap();

    let path = format!("deps/blurhash/e-{c}.png");
    println!("Saving to {path}...");
    blurred_img.save(path).unwrap();
}
