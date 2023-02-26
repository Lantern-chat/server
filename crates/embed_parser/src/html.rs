use std::borrow::Cow;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MetaProperty {
    Name,
    Property,
    Description,
    ItemProp,
    Title,
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Meta<'a> {
    pub content: Cow<'a, str>,
    pub pty: MetaProperty,
    pub property: Cow<'a, str>,
    pub scope: Option<Scope<'a>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Link<'a> {
    pub href: Cow<'a, str>,
    pub rel: LinkType,
    pub ty: Option<Cow<'a, str>>,
    pub title: Option<Cow<'a, str>>,
    pub mime: Option<Cow<'a, str>>,
    pub sizes: Option<[u32; 2]>,
}

#[derive(Default, Debug, Clone, PartialEq, Eq)]
pub struct Scope<'a> {
    pub id: Option<Cow<'a, str>>,
    pub ty: Option<Cow<'a, str>>,
    pub prop: Option<Cow<'a, str>>,
}

impl Meta<'_> {
    pub fn is_valid(&self) -> bool {
        !self.content.is_empty() && !self.property.is_empty()
    }
}

impl Link<'_> {
    pub fn is_valid(&self) -> bool {
        !self.href.is_empty()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Header<'a> {
    Meta(Meta<'a>),
    Link(Link<'a>),
    Scope(Scope<'a>),
}

impl Header<'_> {
    pub fn is_valid(&self) -> bool {
        match self {
            Header::Meta(meta) => meta.is_valid(),
            Header::Link(link) => link.is_valid(),
            Header::Scope(_) => false,
        }
    }
}

use std::cmp::Ordering;

impl PartialOrd for Header<'_> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Header<'_> {
    fn cmp(&self, other: &Self) -> Ordering {
        let key_a = match self {
            Header::Meta(meta) => &meta.property,
            Header::Link(link) => &link.href,
            Header::Scope(_) => &Cow::Borrowed(""),
        };

        let key_b = match other {
            Header::Meta(meta) => &meta.property,
            Header::Link(link) => &link.href,
            Header::Scope(_) => &Cow::Borrowed(""),
        };

        key_a.cmp(key_b)
    }
}

pub type HeaderList<'a> = smallvec::SmallVec<[Header<'a>; 32]>;

pub use crate::regexes::{ATTRIBUTE_RE, META_TAGS};

/// Returns `None` on invalid HTML
pub fn parse_meta<'a>(input: &'a str) -> Option<HeaderList<'a>> {
    let bytes = input.as_bytes();

    let mut res = HeaderList::<'a>::default();
    let mut scope = None;

    for (mut start, mut tag_end) in META_TAGS.find_iter(bytes) {
        // detect tag type and initialize header value
        let mut header = match input.get(start..tag_end) {
            Some("<meta ") => Header::Meta(Meta {
                content: "".into(), // deferred
                pty: MetaProperty::Property,
                property: "".into(),
                scope: scope.clone(),
            }),
            // special case, parse `<title>Title</title>`
            Some("<title>") => {
                let title_start = tag_end;

                if let Some(title_end) = memchr::memmem::find(&bytes[title_start..], b"</title>") {
                    res.push(Header::Meta(Meta {
                        content: input[title_start..(title_start + title_end)].trim().into(),
                        pty: MetaProperty::Title,
                        property: "".into(),
                        scope: scope.clone(),
                    }));
                }

                continue;
            }
            Some("<link ") => Header::Link(Link {
                href: "".into(),
                rel: LinkType::None,
                ty: None,
                title: None,
                mime: None,
                sizes: None,
            }),
            Some(etc) => {
                if etc.starts_with("<div ") {
                    tag_end = start + 4;
                } else if etc.starts_with("<span ") {
                    tag_end = start + 5;
                } else {
                    continue;
                }

                Header::Scope(Scope::default())
            }
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
                let value = html_escape::decode_html_entities(crate::trim_quotes(right));

                match header {
                    Header::Meta(ref mut meta) => {
                        meta.pty = match left {
                            "content" | "href" => {
                                meta.content = value;
                                continue;
                            }
                            "name" => MetaProperty::Name,
                            "property" => MetaProperty::Property,
                            "description" => MetaProperty::Description,

                            // I've seen multiple cases of this around...
                            _ if "itemprop".eq_ignore_ascii_case(left) => MetaProperty::ItemProp,

                            _ => continue,
                        };

                        meta.property = value;
                    }
                    Header::Scope(ref mut scope) => match left {
                        _ if "itemid".eq_ignore_ascii_case(left) => scope.id = Some(value),
                        _ if "itemtype".eq_ignore_ascii_case(left) => scope.ty = Some(value),
                        _ if "itemprop".eq_ignore_ascii_case(left) => scope.prop = Some(value),
                        _ => continue,
                    },
                    Header::Link(ref mut link) => match left {
                        "href" => link.href = value,
                        "type" => link.ty = Some(value),
                        "title" => link.title = Some(value),
                        "rel" => {
                            link.rel = match &*value {
                                "alternate" => LinkType::Alternate,
                                "canonical" => LinkType::Canonical,
                                "external" => LinkType::External,
                                "license" => LinkType::License,
                                "shortlink" => LinkType::Shortlink,
                                "icon" | "shortcut icon" | "apple-touch-icon" => LinkType::Icon,
                                _ => continue,
                            };
                        }
                        // weird, convert to meta
                        _ if "itemprop".eq_ignore_ascii_case(left) => {
                            header = Header::Meta(Meta {
                                content: link.href.clone(),
                                pty: MetaProperty::ItemProp,
                                property: value,
                                scope: scope.clone(),
                            });
                        }
                        // weird, convert to meta
                        "content" => {
                            header = Header::Meta(Meta {
                                content: value,
                                pty: MetaProperty::Property,
                                property: "".into(),
                                scope: scope.clone(),
                            });
                        }
                        _ if link.rel == LinkType::Icon => match left {
                            "sizes" => {
                                link.sizes = Some({
                                    let mut sizes = [0; 2];

                                    for dim in value.split('x').take(2).map(|d| d.parse()).enumerate() {
                                        if let (idx, Ok(value)) = dim {
                                            sizes[idx] = value;
                                        }
                                    }

                                    sizes
                                });
                            }
                            "type" => link.mime = Some(value),
                            _ => continue,
                        },
                        _ => continue,
                    },
                }
            }
        }

        if let Header::Scope(new_scope) = header {
            scope = Some(new_scope);
            continue;
        }

        if header.is_valid() {
            res.push(header);
        }
    }

    res.sort();

    Some(res)
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
