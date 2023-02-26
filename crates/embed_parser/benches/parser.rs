#![allow(deprecated)]

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

//static HTML_FIXTURE: &str = include_str!("../tests/html_fixture.html");

static LINK_HEADER: &str = r#"<https://lantern.chat/api/v1/oembed?format=xml&url=https%3A%2F%2Flantern.chat>; rel="alternate"; title="Testing"; type="text/xml+oembed""#;

use embed_parser::{html, oembed};

fn criterion_benchmark(c: &mut Criterion) {
    let mut g = c.benchmark_group("find_urls");
    //g.bench_with_input("newest", INPUT, |b, x| b.iter(|| msg::find_urls(x)));

    // g.bench_with_input("aho_corasick", INPUT, |b, x| {
    //     b.iter(|| msg::find_urls_aho_corasick(x))
    // });
    // g.bench_with_input("multiple_memchr", INPUT, |b, x| {
    //     b.iter(|| msg::find_urls_multiple_memchr(x))
    // });
    // g.bench_with_input("regex_only", INPUT, |b, x| {
    //     b.iter(|| msg::find_urls_regex_only(x))
    // });
    g.finish();

    //c.bench_function("html_meta", |b| {
    //    let input = black_box(HTML_FIXTURE);
    //    b.iter(|| html::parse_meta(input));
    //});

    c.bench_function("parse_link_header", |b| {
        let input = black_box(LINK_HEADER);
        b.iter(|| oembed::parse_link_header(input));
    });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
