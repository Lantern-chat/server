use criterion::{black_box, criterion_group, criterion_main, Criterion};

use schema::{
    auth::{BotTokenKey, SplitBotToken},
    sf::SnowflakeGenerator,
};
use sdk::models::LANTERN_EPOCH;

fn parse_key<const N: usize>(key: &str) -> [u8; N] {
    let mut out = [0; N];
    hex::decode_to_slice(key, &mut out[..key.len() / 2]).unwrap();
    out
}

fn criterion_benchmark(c: &mut Criterion) {
    c.bench_function("verify_token", |b| {
        let generator = SnowflakeGenerator::new(LANTERN_EPOCH, 0);

        let key: BotTokenKey = parse_key(black_box("5f38e06b42428527d49db9513b251651")).into();

        let token = SplitBotToken::new(&key, generator.gen());

        b.iter(|| token.verify(&key))
    });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
