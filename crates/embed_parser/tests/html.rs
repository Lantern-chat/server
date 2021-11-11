static FIXTURE: &str = include_str!("./html_fixture.html");

use embed_parser::html::parse_meta;

#[test]
fn test_meta() {
    let metas = parse_meta(FIXTURE);

    println!("{:#?}", metas);
}
