use smol_str::SmolStr;

use timestamp::Timestamp;

use models::{Embed, EmbedType};

use crate::html::{Header, LinkType, MetaProperty};
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
pub fn parse_meta_to_embed<'a>(embed: &mut Embed, headers: &[Header<'a>]) -> ExtraFields<'a> {
    let mut extra = ExtraFields::default();

    for header in headers {
        match header {
            Header::Meta(meta) => {
                let content = || Some(SmolStr::from(meta.content));
                let content_int = || meta.content.parse().ok();
                match meta.property {
                    "description" => embed.description = content(),
                    "theme-color" => embed.color = parse_color(meta.content),
                    "og:site_name" => embed.provider_name = content(),
                    "og:url" => embed.provider_url = content(),
                    "og:title" | "twitter:title" => embed.title = content(),
                    "dc:creator" | "article:author" | "book:author" => embed.author = content(),
                    "og:image" | "og:image:secure_url" => {
                        embed.ty = EmbedType::Image;
                        embed.thumbnail_url = content();
                    }
                    "og:video" | "og:video:secure_url" => {
                        embed.ty = EmbedType::Video;
                        embed.thumbnail_url = content();
                    }
                    "og:audio" | "og:audio:secure_url" => {
                        embed.ty = EmbedType::Audio;
                        embed.thumbnail_url = content();
                    }
                    "og:image:width" | "og:video:width" => embed.thumbnail_width = content_int(),
                    "og:image:height" | "og:video:height" => embed.thumbnail_height = content_int(),
                    "og:image:alt" | "og:video:alt" | "og:audio:alt" => embed.alt = content(),
                    "og:image:type" | "og:video:type" | "og:audio:type" => embed.mime = content(),
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

    extra
}

/// Add to/overwrite embed profile with oEmbed data
pub fn parse_oembed_to_embed(embed: &mut Embed, o: OEmbed) -> ExtraFields {
    let mut extra = ExtraFields::default();

    embed.ty = match o.kind {
        OEmbedType::Photo => EmbedType::Image,
        OEmbedType::Video => EmbedType::Video,
        OEmbedType::Rich => EmbedType::Html,
        OEmbedType::Link => EmbedType::Link,
        OEmbedType::Unknown => embed.ty,
    };

    embed.title.overwrite_with(o.title);
    embed.author.overwrite_with(o.author_name);
    embed.author_url.overwrite_with(o.author_url);
    embed.width.overwrite_with(o.width);
    embed.height.overwrite_with(o.height);
    if let Some(url) = o.url {
        embed.url = url;
    }

    embed.thumbnail_url.overwrite_with(o.thumbnail_url);
    embed.thumbnail_width.overwrite_with(o.thumbnail_width);
    embed.thumbnail_height.overwrite_with(o.thumbnail_height);
    embed.html.overwrite_with(o.html);

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
