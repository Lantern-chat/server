static HTML_FIXTURE: &str = include_str!("./derpi/page.html");
static OEMBED_FIXTURE: &str = include_str!("./derpi/oembed.json");

use embed_parser::oembed::OEmbed;
use models::Embed;

#[test]
fn test_parse_yt() {
    let oembed: OEmbed = serde_json::from_str(OEMBED_FIXTURE).unwrap();

    let mut embed = Embed::default();

    let headers = embed_parser::html::parse_meta(HTML_FIXTURE).unwrap();

    let extra = embed_parser::embed::parse_meta_to_embed(&mut embed, &headers);

    let extra2 = embed_parser::embed::parse_oembed_to_embed(&mut embed, oembed);

    println!("{:#?}", embed);

    println!("{}", serde_json::to_string_pretty(&embed).unwrap());
}
