use smol_str::SmolStr;

use timestamp::Timestamp;

use sdk::models::{Embed, EmbedMedia, EmbedType};

use crate::html::{Header, LinkType};
use crate::oembed::{OEmbed, OEmbedFormat, OEmbedLink, OEmbedType};

#[derive(Default)]
pub struct ExtraFields<'a> {
    pub expires: Option<Timestamp>,
    pub link: Option<OEmbedLink<'a>>,
}

fn parse_color(mut color: &str) -> Option<i32> {
    color = color.trim();

    if color.starts_with('#') {}

    None
}

/// Build an initial embed profile from HTML meta tags
///
/// NOTE: HEADERS MUST BE SORTED BY PROPERTY NAME FOR OPTIMAL RESULTS
pub fn parse_meta_to_embed<'a>(embed: &mut Embed, headers: &[Header<'a>]) -> ExtraFields<'a> {
    let mut extra = ExtraFields::default();

    for header in headers {
        match header {
            Header::Meta(meta) => {
                let raw_content = || SmolStr::from(meta.content);
                let content = || Some(SmolStr::from(meta.content));
                let content_int = || meta.content.parse().ok();

                macro_rules! get {
                    ($e:ident) => {
                        embed.$e.get_or_insert_with(Default::default)
                    };
                }

                match meta.property {
                    "description" => embed.description = content(),
                    "theme-color" => embed.color = parse_color(meta.content),
                    "og:site_name" => embed.provider.name = content(),
                    // canonical URL?
                    // "og:url" => embed.url = content(),
                    "og:title" | "twitter:title" => embed.title = content(),
                    "dc:creator" | "article:author" | "book:author" => get!(author).name = raw_content(),

                    "og:image" | "og:image:url" | "og:image:secure_url" => get!(image).url = raw_content(),
                    // don't let the twitter image overwrite og images
                    "twitter:image" => match embed.image {
                        Some(ref mut image) if image.url.is_empty() => image.url = raw_content(),
                        None => get!(image).url = raw_content(),
                        _ => {}
                    },
                    "og:video" | "og:video:secure_url" => get!(video).url = raw_content(),
                    "og:audio" | "og:audio:secure_url" => get!(audio).url = raw_content(),

                    "og:image:width" => get!(image).width = content_int(),
                    "og:video:width" => get!(video).width = content_int(),
                    "music:duration" => get!(audio).width = content_int(),

                    "og:image:height" => get!(image).height = content_int(),
                    "og:video:height" => get!(video).height = content_int(),

                    "og:image:type" => get!(image).mime = content(),
                    "og:video:type" => get!(video).mime = content(),
                    "og:audio:type" => get!(audio).mime = content(),

                    "og:image:alt" => get!(image).alt = content(),
                    "og:video:alt" => get!(video).alt = content(),
                    "og:audio:alt" => get!(audio).alt = content(),

                    //"profile:first_name" | "profile:last_name" | "profile:username" | "profile:gender" => {
                    //    embed.fields.push(EmbedField {
                    //          inline: true,
                    //    })
                    //}
                    _ => {}
                }
            }
            Header::Link(link) if link.rel == LinkType::Alternate => {
                let ty = match link.ty {
                    Some(ty) if ty.contains("oembed") => ty,
                    _ => continue,
                };

                match extra.link {
                    Some(ref mut existing) => {
                        if ty.contains("json") && existing.format == OEmbedFormat::XML {
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

    if embed.image.is_some() {
        embed.ty = EmbedType::Image;
    }

    if embed.audio.is_some() {
        embed.ty = EmbedType::Audio;
    }

    if embed.video.is_some() {
        embed.ty = EmbedType::Video;
    }

    if embed.object.is_some() {
        embed.ty = EmbedType::Html;
    }

    extra
}

/// Add to/overwrite embed profile with oEmbed data
pub fn parse_oembed_to_embed(embed: &mut Embed, o: OEmbed) -> ExtraFields {
    macro_rules! get {
        ($e:ident) => {
            embed.$e.get_or_insert_with(Default::default)
        };
    }

    let mut extra = ExtraFields::default();

    embed.ty = match o.kind {
        OEmbedType::Photo => EmbedType::Image,
        OEmbedType::Video => EmbedType::Video,
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

    embed.title.overwrite_with(o.title);
    embed.provider.name.overwrite_with(o.provider_name);
    embed.provider.url.overwrite_with(o.provider_url);

    let media = match o.kind {
        OEmbedType::Photo => get!(image),
        OEmbedType::Video => get!(video),
        _ => get!(object),
    };

    let mut mime = media.mime.take();
    let mut overwrite = false;

    if let Some(html) = o.html {
        match mime {
            Some(ref mime) if mime == "text/html" => {}
            _ => match parse_embed_html_src(&html) {
                Some(src) => {
                    media.url = src;
                    mime = Some(parse_embed_html_type(&html).unwrap_or(SmolStr::new_inline("text/html")));
                    overwrite = true;
                }
                _ => {}
            },
        }
    } else if let Some(url) = o.url {
        media.url = url;
        mime = None; // unknown
        overwrite = true;
    }

    media.mime = mime;

    if overwrite {
        media.width.overwrite_with(o.width);
        media.height.overwrite_with(o.height);
    }

    if let Some(thumbnail_url) = o.thumbnail_url {
        let mut thumb = EmbedMedia::default();

        thumb.url = thumbnail_url;
        thumb.width = o.thumbnail_width;
        thumb.height = o.thumbnail_height;

        thumb.mime = None; // unknown from here

        embed.thumb = Some(thumb);
    }

    if let Some(cache_age) = o.cache_age {
        extra.expires = Some(Timestamp::now_utc() + std::time::Duration::from_secs(cache_age as u64));
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
