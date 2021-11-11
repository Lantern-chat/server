#![allow(deprecated)]

use criterion::{black_box, criterion_group, criterion_main, Criterion, ParameterizedBenchmark};

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

static HTML_FIXTURE: &str = include_str!("../tests/html_fixture.html");

use embed_parser::{html, msg};

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

    c.bench_function("html_meta", |b| {
        let input = black_box(HTML_FIXTURE);
        b.iter(|| html::parse_meta(input));
    });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
