//use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OEmbedFormat {
    XML,
    JSON,
}

#[derive(Debug, Clone, PartialEq)]
pub struct OEmbedLink<'a> {
    pub url: &'a str,
    pub title: Option<&'a str>,
    pub format: OEmbedFormat,
}

pub type LinkList<'a> = smallvec::SmallVec<[OEmbedLink<'a>; 1]>;

pub fn parse_link_header<'a>(header: &'a str) -> LinkList<'a> {
    let mut res = LinkList::default();

    // multiple links can be comma-separated
    'links: for link in header.split(',') {
        let mut parts = link.split(";").map(str::trim);

        let url = match parts.next() {
            Some(url) if url.starts_with("<http") && url.ends_with('>') => &url[1..url.len() - 1],
            _ => continue,
        };

        let mut link = OEmbedLink {
            url,
            title: None,
            format: OEmbedFormat::JSON,
        };

        while let Some(part) = parts.next() {
            let (left, right) = match part.split_once('=') {
                Some(v) => v,
                None => continue 'links,
            };

            if left == "type" && right.contains("xml") {
                link.format = OEmbedFormat::XML;
                continue;
            }

            let right = crate::trim_quotes(right);

            match left {
                "title" => {
                    link.title = Some(right);
                }
                "rel" if right != "alternate" => continue 'links,
                _ => continue,
            }
        }

        res.push(link);
    }

    res
}
