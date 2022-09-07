#![allow(unused_imports)]

use md_utils::{scan_markdown, Span, SpanList, SpanType};

static INPUT: &str = r#"
```rust
fn test() {
    println!("{}", "`https://code.com");
    ||lol
}
```
<http://escaped.com>
shttps://test.com

<http://whatever.com>

||https://spoiler.com||

`https://lol.cats`

http://last.net/test.php?query=true#hash

||spoiler|| `inline_code_1``inline_code_2` ```rust
block code
https://code.com
```

https://google.com

<http://bing.com>

||https://google.com/spoiler||

||`spoilered_code`||||second-spoiler||

<@12345><@12345>

<:test:1234>
<:test:1234455:>
<:tewsdtsdgsdg>
"#;

fn main() {
    let parsed = scan_markdown(INPUT);

    println!("--------\n{}\n--------", INPUT);

    for span in &parsed {
        let sub = std::str::from_utf8(&INPUT.as_bytes()[span.range()]).unwrap();

        let kind = format!(
            "{:?}{}",
            span.kind(),
            if span.kind() != SpanType::Spoiler && md_utils::is_spoilered(&parsed, span.start()) {
                " *"
            } else {
                ""
            }
        );

        println!("{:15} {:5}..{:<5} -> {:?}", kind, span.start(), span.end(), sub);
    }
}
