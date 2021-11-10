use embed_parser::msg::{find_urls, is_free, Url};

#[test]
fn test_is_free() {
    assert!(is_free("`test` https", 6));
    assert!(!is_free("<test>", 1));
    assert!(!is_free("||test||", 4));
    assert!(!is_free("`https`", 3));
    assert!(!is_free(
        r#"
    ```rust
    fn main() {}
    ```
    "#,
        10
    ));
}

#[test]
fn test_find_urls() {
    assert_eq!(
        find_urls("https://test.com").as_slice(),
        &[Url {
            secure: true,
            url: "https://test.com"
        }]
    );

    assert_eq!(
        find_urls("http://test.com").as_slice(),
        &[Url {
            secure: false,
            url: "http://test.com"
        }]
    );

    assert_eq!(find_urls("<http://test.com>").as_slice(), &[]);

    assert_eq!(
        find_urls(
            r#"
```rust
fn test() {
    println!("{}", "https://test.com");
}
```

||http://test.com||
"#
        )
        .as_slice(),
        &[]
    );

    assert_eq!(
        find_urls(
            r#"
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
"#
        )
        .as_slice(),
        &[
            Url {
                secure: true,
                url: "https://test.com"
            },
            Url {
                secure: false,
                url: "http://last.net/test.php?query=true#hash"
            }
        ]
    );

    assert_eq!(
        find_urls(
            r#"
||```
https://test.com ||
```
https://another.com
||
"#
        )
        .as_slice(),
        &[]
    );
}
