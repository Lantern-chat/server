#![allow(deprecated)]

use criterion::{criterion_group, criterion_main, Criterion, ParameterizedBenchmark};

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

use embed_parser::msg;

fn criterion_benchmark(c: &mut Criterion) {
    c.bench(
        "find_urls",
        ParameterizedBenchmark::new(
            "newest",
            |b, x| {
                b.iter(|| msg::find_urls(x));
            },
            vec![INPUT],
        )
        .with_function("aho_corasick", |b, x| {
            b.iter(|| msg::find_urls_aho_corasick(x));
        })
        .with_function("multiple_memchr", |b, x| {
            b.iter(|| msg::find_urls_multiple_memchr(x));
        })
        .with_function("regex_only", |b, x| b.iter(|| msg::find_urls_regex_only(x))),
    );
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
