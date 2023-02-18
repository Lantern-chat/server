use sdk::models::*;

pub fn resolve_relative(root: &str, https: bool, embed: &mut EmbedV1) {
    embed.visit_media_mut(|media| {
        if media.url.starts_with("https://") || media.url.starts_with("http://") {
            return;
        }

        if media.url.starts_with(".") {
            // TODO
        }

        let old = media.url.as_str();

        media.url = 'media_url: {
            let mut url = root.to_owned();

            // I've seen this before, where "https://" is replaced with "undefined//"
            for prefix in ["undefined//", "//"] {
                if old.starts_with(prefix) {
                    url = if https { "https://" } else { "http://" }.to_owned();
                    url += &old[prefix.len()..];
                    break 'media_url url.into();
                }
            }

            url += "/";
            url += &old;
            url.into()
        };
    });
}

pub fn fix_embed(embed: &mut EmbedV1) {
    // get rid of invalid images introduced through bad embeds
    {
        if let Some(ref img) = embed.img {
            if let Some(ref mime) = img.mime {
                if !mime.starts_with("image") {
                    embed.img = None;
                }
            }
        }

        for field in &mut embed.fields {
            if let Some(ref img) = field.img {
                if let Some(ref mime) = img.mime {
                    if !mime.starts_with("image") {
                        field.img = None;
                    }
                }
            }
        }
    }

    // redundant canonical
    match (&embed.can, &embed.url) {
        (Some(can), Some(url)) if can == url => {
            embed.can = None;
        }
        _ => {}
    }

    // redundant thumbnail
    match (&embed.img, &embed.thumb) {
        (Some(img), Some(thumb)) if thumb.url == img.url => {
            embed.thumb = None;
        }
        _ => {}
    }

    embed.visit_text_mut(|text| {
        use std::borrow::Cow;

        if let Cow::Owned(new_text) = html_escape::decode_html_entities(text.as_str()) {
            *text = new_text.into();
        }
    });

    embed.fields.retain(|f| !EmbedField::is_empty(f));
}
