use criterion::{black_box, criterion_group, criterion_main, Criterion};

static INPUT: &str = r#"
```rust
fn test() {
    println!("{}", "`https://code.com");
    ||lol
}
```
<http://escaped.com>
https://test.com

<http://whatever.com>

||https://spoiler.com||

`https://lol.cats`

http://last.net/test.php?query=true#hash
"#;

use embed_parser::msg::{find_urls, find_urls2};

fn criterion_benchmark(c: &mut Criterion) {
    c.bench_function("find_urls", |b| {
        let input = black_box(INPUT);

        b.iter(|| find_urls(input));
    });

    c.bench_function("find_urls2", |b| {
        let input = black_box(INPUT);

        b.iter(|| find_urls2(input));
    });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
