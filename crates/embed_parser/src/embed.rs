use smol_str::SmolStr;

use sdk::models::{EmbedField, EmbedMedia, EmbedType, EmbedV1};

use crate::html::{Header, LinkType, MetaProperty};
use crate::oembed::{OEmbed, OEmbedFormat, OEmbedLink, OEmbedType};

pub struct ExtraFields<'a> {
    pub max_age: u64,
    pub link: Option<OEmbedLink<'a>>,
}

impl Default for ExtraFields<'_> {
    fn default() -> Self {
        ExtraFields {
            max_age: 60 * 15, // 15 minutes
            link: None,
        }
    }
}

fn parse_color(color: &str) -> Option<u32> {
    match csscolorparser::parse(color) {
        Err(_) => None,
        Ok(color) => Some(u32::from_le_bytes(color.to_rgba8())),
    }
}

/// Build an initial embed profile from HTML meta tags
///
/// NOTE: HEADERS MUST BE SORTED BY PROPERTY NAME FOR OPTIMAL RESULTS
pub fn parse_meta_to_embed<'a>(embed: &mut EmbedV1, headers: &[Header<'a>]) -> ExtraFields<'a> {
    let mut extra = ExtraFields::default();

    #[derive(Default, Clone, Copy)]
    struct Misc<'a> {
        label: Option<&'a str>,
        data: Option<&'a str>,
    }

    let mut misc: [Misc; 4] = [Misc::default(); 4];

    for header in headers {
        match header {
            Header::Meta(meta) => {
                let raw_content = || SmolStr::from(&meta.content);
                let content = || Some(SmolStr::from(&meta.content));
                let content_int = || meta.content.parse().ok();

                macro_rules! get {
                    ($e:ident) => {
                        embed.$e.get_or_insert_with(Default::default)
                    };
                }

                match meta.property {
                    // special property for <title></title> values
                    "" if meta.pty == MetaProperty::Title => {
                        if embed.title.is_none() {
                            embed.title = content();
                        }
                    }

                    "description" | "og:description" | "twitter:description" => embed.desc = content(),
                    "theme-color" | "msapplication-TileColor" => embed.col = parse_color(&meta.content),
                    "og:site_name" => embed.pro.name = content(),
                    // TODO: This isn't quite correct, but good enough most of the time
                    "og:url" => embed.can = content(),
                    "title" | "og:title" | "twitter:title" => embed.title = content(),
                    "dc:creator" | "article:author" | "book:author" => get!(author).name = raw_content(),

                    "og:image" | "og:image:url" | "og:image:secure_url" => get!(img).url = raw_content(),
                    // don't let the twitter image overwrite og images
                    "twitter:image" => match embed.img {
                        Some(ref mut image) if image.url.is_empty() => image.url = raw_content(),
                        None => get!(img).url = raw_content(),
                        _ => {}
                    },
                    "og:video" | "og:video:secure_url" => get!(vid).url = raw_content(),
                    "og:audio" | "og:audio:secure_url" => get!(audio).url = raw_content(),

                    "og:image:width" => get!(img).w = content_int(),
                    "og:video:width" => get!(vid).w = content_int(),
                    "music:duration" => get!(audio).w = content_int(),

                    "og:image:height" => get!(img).h = content_int(),
                    "og:video:height" => get!(vid).h = content_int(),

                    "og:image:type" => get!(img).mime = content(),
                    "og:video:type" => get!(vid).mime = content(),
                    "og:audio:type" => get!(audio).mime = content(),

                    "og:image:alt" => get!(img).alt = content(),
                    "og:video:alt" => get!(vid).alt = content(),
                    "og:audio:alt" => get!(audio).alt = content(),

                    "og:ttl" => match content_int() {
                        None => {}
                        Some(ttl) => extra.max_age = ttl as u64,
                    },

                    "twitter:label1" | "twitter:label2" | "twitter:label3" | "twitter:label4" => {
                        let idx = meta.property.as_bytes()[meta.property.len() - 1] - b'0';
                        misc[idx as usize - 1].label = Some(&meta.content);
                    }

                    "twitter:data1" | "twitter:data2" | "twitter:data3" | "twitter:data4" => {
                        let idx = meta.property.as_bytes()[meta.property.len() - 1] - b'0';
                        misc[idx as usize - 1].data = Some(&meta.content);
                    }

                    _ if meta.property.eq_ignore_ascii_case("rating") => parse_rating(embed, &meta.content),

                    // Twitter uses these for multi-image posts
                    // FIXME: Can also include images from replies!
                    _ if meta.pty == MetaProperty::ItemProp
                        && meta.property.eq_ignore_ascii_case("contenturl") =>
                    {
                        // reasonable limit for embedding
                        if embed.fields.len() < 4 {
                            embed.fields.push(EmbedField {
                                name: "".into(),
                                value: "".into(),
                                b: false,
                                img: Some(Box::new(EmbedMedia {
                                    url: raw_content(),
                                    ..EmbedMedia::default()
                                })),
                            });
                        }
                    }

                    //"profile:first_name" | "profile:last_name" | "profile:username" | "profile:gender" => {
                    //    embed.fields.push(EmbedField {
                    //          inline: true,
                    //    })
                    //}
                    _ => {}
                }
            }
            Header::Link(link) if link.rel == LinkType::Canonical => {
                embed.can = Some(link.href.into());
            }
            Header::Link(link) if link.rel == LinkType::Alternate => {
                let ty = match link.ty {
                    Some(ty) if ty.contains("oembed") => ty,
                    _ => continue,
                };

                match extra.link {
                    Some(ref mut existing) => {
                        if ty.contains("json") {
                            existing.url = link.href;
                            existing.title = link.title;
                            existing.format = OEmbedFormat::JSON;
                        }
                    }
                    None => {
                        extra.link = Some(OEmbedLink {
                            url: link.href,
                            title: link.title,
                            format: if ty.contains("xml") { OEmbedFormat::XML } else { OEmbedFormat::JSON },
                        });
                    }
                }
            }
            _ => {}
        }
    }

    for m in misc {
        if let (Some(label), Some(data)) = (m.label, m.data) {
            if label.eq_ignore_ascii_case("rating") {
                parse_rating(embed, data)
            }

            // TODO: Maybe recurse to handle more arbitrary properties?
        }
    }

    determine_embed_type(embed);

    extra
}

fn determine_embed_type(embed: &mut EmbedV1) {
    if embed.img.is_some() {
        embed.ty = EmbedType::Img;
    }

    if embed.audio.is_some() {
        embed.ty = EmbedType::Audio;
    }

    if embed.vid.is_some() {
        embed.ty = EmbedType::Vid;
    }

    if embed.obj.is_some() {
        embed.ty = EmbedType::Html;
    }
}

pub fn parse_rating(embed: &mut EmbedV1, rating: &str) {
    // NOTE: In case of multiple tags, this is additive
    embed.a |= crate::regexes::ADULT_RATING.is_match(rating.as_bytes());
}

/// Add to/overwrite embed profile with oEmbed data
pub fn parse_oembed_to_embed(embed: &mut EmbedV1, o: OEmbed) -> ExtraFields {
    macro_rules! get {
        ($e:ident) => {
            embed.$e.get_or_insert_with(Default::default)
        };
    }

    let mut extra = ExtraFields::default();

    embed.ty = match o.kind {
        OEmbedType::Photo => EmbedType::Img,
        OEmbedType::Video => EmbedType::Vid,
        OEmbedType::Rich => EmbedType::Html,
        OEmbedType::Link => EmbedType::Link,
        OEmbedType::Unknown => embed.ty,
    };

    if o.author_name.is_some() || o.author_url.is_some() {
        let author = get!(author);

        author.url.overwrite_with(o.author_url);
        if let Some(author_name) = o.author_name {
            author.name = author_name;
        }
    }

    // QUIRK: Sometimes oEmebed returns a bad title
    // that's just a prefix of the meta tags title
    if let Some(title) = o.title {
        match embed.title {
            Some(ref t) if t.starts_with(title.as_str()) => {}
            _ => embed.title = Some(title),
        }
    }

    embed.pro.name.overwrite_with(o.provider_name);
    embed.pro.url.overwrite_with(o.provider_url);

    if embed.ty == EmbedType::Link {
        determine_embed_type(embed);
    }

    let media = match o.kind {
        OEmbedType::Photo => get!(img),
        OEmbedType::Video => get!(vid),
        _ => get!(obj),
    };

    let mut mime = media.mime.take();
    let mut overwrite = false;

    if let Some(html) = o.html {
        match mime {
            Some(ref mime) if mime == "text/html" => {}
            _ => {
                if let Some(src) = parse_embed_html_src(&html) {
                    media.url = src;
                    mime = Some(parse_embed_html_type(&html).unwrap_or(SmolStr::new_inline("text/html")));
                    overwrite = true;
                }
            }
        }
    } else if let Some(url) = o.url {
        media.url = url;
        mime = None; // unknown
        overwrite = true;
    }

    media.mime = mime;

    if overwrite {
        media.w = o.width;
        media.h = o.height;
    }

    if let Some(thumbnail_url) = o.thumbnail_url {
        let mut thumb = Box::new(EmbedMedia::default());

        thumb.url = thumbnail_url;
        thumb.w = o.thumbnail_width;
        thumb.h = o.thumbnail_height;

        thumb.mime = None; // unknown from here

        embed.thumb = Some(thumb);
    }

    if let Some(cache_age) = o.cache_age {
        extra.max_age = cache_age as u64;
    }

    extra
}

pub trait OptionExt<T> {
    fn overwrite_with(&mut self, value: Option<T>);
}

impl<T> OptionExt<T> for Option<T> {
    #[inline]
    fn overwrite_with(&mut self, value: Option<T>) {
        if value.is_some() {
            *self = value;
        }
    }
}

fn parse_embed_html_src(html: &str) -> Option<SmolStr> {
    let mut start = memchr::memmem::find(html.as_bytes(), b"src=\"http")?;

    // strip src=" prefix
    start += "src=\"".len();

    let end = start + memchr::memchr(b'"', &html.as_bytes()[start..])?;

    let src = &html[start..end];

    if memchr::memmem::find(src.as_bytes(), b"://").is_none() {
        return None;
    }

    Some(SmolStr::from(src))
}

fn parse_embed_html_type(html: &str) -> Option<SmolStr> {
    let mut start = memchr::memmem::find(html.as_bytes(), b"type=\"")?;

    start += "type=\"".len(); // strip prefix

    let end = start + memchr::memchr(b'"', &html.as_bytes()[start..])?;

    let ty = &html[start..end];

    // mime type e.g.: image/png
    if memchr::memchr(b'/', ty.as_bytes()).is_none() {
        return None;
    }

    Some(SmolStr::from(ty))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_embed_html() {
        let fixture = "<object width=\"425\" height=\"344\">
        <param name=\"movie\" value=\"https://www.youtube.com/v/M3r2XDceM6A&fs=1\"></param>
        <param name=\"allowFullScreen\" value=\"true\"></param>
        <param name=\"allowscriptaccess\" value=\"always\"></param>
        <embed src=\"https://www.youtube.com/v/M3r2XDceM6A&fs=1\"
            type=\"application/x-shockwave-flash\" width=\"425\" height=\"344\"
            allowscriptaccess=\"always\" allowfullscreen=\"true\"></embed>
        </object>";

        let src = parse_embed_html_src(fixture);
        let ty = parse_embed_html_type(fixture);

        assert_eq!(
            src.as_ref().map::<&str, _>(|s| &s),
            Some("https://www.youtube.com/v/M3r2XDceM6A&fs=1")
        );

        assert_eq!(
            ty.as_ref().map::<&str, _>(|s| &s),
            Some("application/x-shockwave-flash")
        );
    }
}
