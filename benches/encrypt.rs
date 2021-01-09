#![allow(unused)]

use criterion::{black_box, criterion_group, criterion_main, Criterion, ParameterizedBenchmark};

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);

use chacha20::cipher::{NewStreamCipher, SyncStreamCipher, SyncStreamCipherSeek};
use chacha20::{ChaCha20, Key, Nonce};

// `aes` crate provides AES block cipher implementation
type Aes256Ctr = ctr::Ctr128<aes::Aes256>;

fn criterion_benchmark(c: &mut Criterion) {
    let fixture = std::fs::read("benches/fixtures/test.png").unwrap();

    c.bench(
        "encrypt",
        ParameterizedBenchmark::new(
            "chacha20",
            |b, data| {
                let chacha_key = Key::from_slice(b"an example very very secret key.");
                let chacha_nonce = Nonce::from_slice(b"secret nonce");

                b.iter_with_large_setup(
                    || (ChaCha20::new(&chacha_key, &chacha_nonce), data.clone()),
                    |(mut cipher, mut data)| cipher.apply_keystream(&mut data),
                )
                // test
            },
            vec![fixture],
        )
        .with_function("aes", |b, data| {
            let aes_key = b"very secret key.very secret key.";
            let aes_nonce = b"and secret nonce";

            b.iter_with_large_setup(
                || {
                    (
                        Aes256Ctr::new(aes_key.into(), aes_nonce.into()),
                        data.clone(),
                    )
                },
                |(mut cipher, mut data)| cipher.apply_keystream(&mut data),
            )
        }),
    );
}
