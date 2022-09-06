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

||spoiler|| `inline_code_1``inline_code````rust
block code
https://code.com
```

https://google.com

<http://bing.com>

||https://google.com/spoiler||

||`spoilered_code`||

<@12345><@12345>
"#;

fn criterion_benchmark(c: &mut Criterion) {
    c.bench(
        "find_urls",
        ParameterizedBenchmark::new(
            "newest",
            |b, x| b.iter(|| md_utils::scan_markdown(x)),
            vec![INPUT],
        ),
    );
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
