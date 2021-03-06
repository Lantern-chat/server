use embed_parser::oembed::{parse_link_header, OEmbedFormat, OEmbedLink};

#[test]
fn test_parse_link_header() {
    assert_eq!(
        parse_link_header(r#"<https://test.com>; rel="alternate"; title="test""#).as_slice(),
        &[OEmbedLink {
            url: "https://test.com",
            title: Some("test"),
            format: OEmbedFormat::JSON,
        }]
    );

    assert_eq!(
        parse_link_header(r#"<https://test.com>; rel="alternate"; title="Testing"; type="text/xml+oembed", <https://test.com>; rel="alternate""#)
            .as_slice(),
        &[OEmbedLink {
            url: "https://test.com",
            title: Some("Testing"),
            format: OEmbedFormat::XML,
        }, OEmbedLink {
            url: "https://test.com",
            title: None,
            format: OEmbedFormat::JSON,
        }]
    );

    assert_eq!(
        parse_link_header(r#"<https://lantern.chat/api/v1/json/oembed?url=%2Fimages%2F1234>; rel="alternate"; type="application/json+oembed""#).as_slice(),
        &[
            OEmbedLink {
                url: "https://lantern.chat/api/v1/json/oembed?url=%2Fimages%2F1234",
                title: None,
                format: OEmbedFormat::JSON
            }
        ]
    );
}
