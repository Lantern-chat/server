#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MetaProperty {
    Name,
    Property,
    Description,
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

use memchr::memmem::find;

/// Returns `None` on invalid HTML
pub fn parse_meta<'a>(mut input: &'a str) -> Option<HeaderList<'a>> {
    let mut res = HeaderList::default();

    // constrain search region to <head></head> delimiters
    let head_start = find(input.as_bytes(), b"<head>")? + "<head>".len();
    input = &input[head_start..];
    let head_end = find(input.as_bytes(), b"</head>")?;
    input = &input[..head_end];

    let bytes = input.as_bytes();

    let tag_start_iter = memchr::memchr_iter(b'<', bytes);

    const TAG_LEN: usize = "<meta ".len();

    for mut start in tag_start_iter {
        let tag_end = start + TAG_LEN;

        // detect tag type and initialize header value
        let mut header = match input.get(start..tag_end) {
            Some("<meta ") => Header::Meta(Meta {
                content: "",
                pty: MetaProperty::Property,
                property: "",
            }),
            Some("<link ") => Header::Link(Link {
                href: "",
                rel: LinkType::None,
                ty: None,
            }),
            Some(_) => continue,
            None => return None,
        };

        start = tag_end; // skip to end of opening tag

        // find end of tag, like <meta whatever="" >
        let end = memchr::memchr(b'>', &bytes[start..])? + start;
        let meta_inner = &input[start..end];

        // name="" content=""
        for part in meta_inner.split_ascii_whitespace() {
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
                            _ => continue,
                        };

                        meta.property = value;
                    }
                    Header::Link(ref mut link) => match left {
                        "href" => link.href = value,
                        "type" => link.ty = Some(value),
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

    Some(res)
}
