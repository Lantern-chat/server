#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MetaProperty {
    Name,
    Property,
    Description,
    ItemProp,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LinkType {
    None,
    Alternate,
    //Author,
    //Bookmark,
    Canonical,
    External,
    //DnsPrefetch,
    //Help,
    Icon,
    License,
    Shortlink,
    //Stylesheet,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Meta<'a> {
    pub content: &'a str,
    pub pty: MetaProperty,
    pub property: &'a str,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Link<'a> {
    pub href: &'a str,
    pub rel: LinkType,
    pub ty: Option<&'a str>,
    pub title: Option<&'a str>,
}

impl Meta<'_> {
    pub const fn is_valid(&self) -> bool {
        !self.content.is_empty() && !self.property.is_empty()
    }
}

impl Link<'_> {
    pub const fn is_valid(&self) -> bool {
        !self.href.is_empty()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Header<'a> {
    Meta(Meta<'a>),
    Link(Link<'a>),
}

impl Header<'_> {
    pub const fn is_valid(&self) -> bool {
        match self {
            Header::Meta(meta) => meta.is_valid(),
            Header::Link(link) => link.is_valid(),
        }
    }
}

pub type HeaderList<'a> = smallvec::SmallVec<[Header<'a>; 32]>;

pub use crate::regexes::{ATTRIBUTE_RE, META_TAGS};

/// Returns `None` on invalid HTML
pub fn parse_meta<'a>(input: &'a str) -> Option<HeaderList<'a>> {
    let mut res = HeaderList::default();

    // NOTE: Too many sites completely ignore the spec, so we cannot constrain to <head>, sadly.
    // in fact, <head> may not even exist, yet <meta> tags are still somewhere.

    // constrain search region to <head></head> delimiters
    //let head_start = find(input.as_bytes(), b"<head")? + "<head".len();
    //input = &input[head_start..];
    //if let Some(head_end) = memchr::memmem::rfind(input.as_bytes(), b"</body>") {
    //    input = &input[..head_end];
    //}

    let bytes = input.as_bytes();

    for (mut start, tag_end) in META_TAGS.find_iter(bytes) {
        // detect tag type and initialize header value
        let mut header = match input.get(start..tag_end) {
            Some("<meta ") => Header::Meta(Meta {
                content: "",
                pty: MetaProperty::Property,
                property: "",
            }),
            // special case, parse `<title>Title</title>`
            Some("<title>") => {
                let title_start = tag_end;

                if let Some(title_end) = memchr::memmem::find(&bytes[title_start..], b"</title>") {
                    res.push(Header::Meta(Meta {
                        content: &input[title_start..(title_start + title_end)],
                        pty: MetaProperty::Property,
                        property: "html_title",
                    }));
                }

                continue;
            }
            Some("<link ") => Header::Link(Link {
                href: "",
                rel: LinkType::None,
                ty: None,
                title: None,
            }),
            // hit actual HTML, bail
            Some("<div") => break,
            _ => continue,
        };

        start = tag_end; // skip to end of opening tag

        // find end of tag, like <meta whatever="" >
        let end = match memchr::memchr(b'>', &bytes[start..]) {
            Some(end) => end + start,
            None => continue,
        };
        let meta_inner = &input[start..end];

        // name="" content=""
        for (m0, m1) in ATTRIBUTE_RE.find_iter(meta_inner.as_bytes()) {
            let part = &meta_inner[m0..m1];

            // name=""
            if let Some((left, right)) = part.split_once('=') {
                let value = crate::trim_quotes(right);

                match header {
                    Header::Meta(ref mut meta) => {
                        meta.pty = match left {
                            "content" => {
                                meta.content = value;
                                continue;
                            }
                            "name" => MetaProperty::Name,
                            "property" => MetaProperty::Property,
                            "description" => MetaProperty::Description,
                            "itemprop" | "ItemProp" => MetaProperty::ItemProp,
                            _ => continue,
                        };

                        meta.property = value;
                    }
                    Header::Link(ref mut link) => match left {
                        "href" => link.href = value,
                        "type" => link.ty = Some(value),
                        "title" => link.title = Some(value),
                        "rel" => {
                            link.rel = match value {
                                "alternate" => LinkType::Alternate,
                                "canonical" => LinkType::Canonical,
                                "external" => LinkType::External,
                                "icon" => LinkType::Icon,
                                "license" => LinkType::License,
                                "shortlink" => LinkType::Shortlink,
                                _ => continue,
                            };
                        }
                        _ => continue,
                    },
                }
            }
        }

        if header.is_valid() {
            res.push(header);
        }
    }

    // ensure properties are sorted lexicongraphically (href doesn't matter but needs a value anyway)
    res.sort_by_key(|h| match h {
        Header::Meta(meta) => meta.property,
        Header::Link(link) => link.href,
    });

    Some(res)
}

// NOT NEEDED, XML/HTML specification requires there be no whitespace between opening/closing symbols and tag name

pub struct HeadEndFinder {
    pos: usize,
    needle: usize,
}

const NEEDLE: [u8; 6] = *b"</head";

impl HeadEndFinder {
    pub const fn new() -> Self {
        HeadEndFinder { pos: 0, needle: 0 }
    }

    pub fn found(&self) -> Option<usize> {
        (self.needle == NEEDLE.len()).then_some(self.pos)
    }

    pub fn increment(&mut self, input: &[u8]) {
        while self.pos < input.len() && self.needle != NEEDLE.len() {
            let c = input[self.pos];

            // skip whitespace
            if !c.is_ascii_whitespace() {
                if c == NEEDLE[self.needle] {
                    self.needle += 1;
                } else {
                    // set back to 0 or 1 if matched first byte
                    self.needle = (c == NEEDLE[0]) as usize;
                }
            }

            self.pos += 1;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_attribute_regex_size() {
        println!(
            "{}",
            ATTRIBUTE_RE.forward().memory_usage() + ATTRIBUTE_RE.reverse().memory_usage()
        );
    }
}
