#![allow(deprecated)]

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use util::hex::HexidecimalInt;

fn criterion_benchmark(c: &mut Criterion) {
    let t = 119324240026741659787093958279368883115u128;

    //c.bench_function("encode_b62_u128", |b| {
    //    b.iter(|| util::base62::encode128(black_box(t)))
    //});

    c.bench_function("encode_hex_u128", |b| {
        b.iter(|| HexidecimalInt(black_box(t)).to_string())
    });

    c.bench_function("encode_base64_u128", |b| {
        b.iter(|| util::base64::encode_u128(black_box(u128::MAX)))
    });

    ////////////////////
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
