static FIXTURE: &str = include_str!("./html_fixture.html");

use embed_parser::html::parse_meta;

#[test]
fn test_meta() {
    let metas = parse_meta(FIXTURE);

    println!("{:#?}", metas);
}

//#[test]
//fn test_find_head() {
//    let mut finder = HeadEndFinder::new();
//
//    finder.increment(FIXTURE.as_bytes());
//
//    assert!(finder.found().is_some());
//}
