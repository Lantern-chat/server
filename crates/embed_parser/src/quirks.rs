use sdk::models::*;

pub fn resolve_relative(root: &str, https: bool, embed: &mut EmbedV1) {
    embed.visit_media_mut(|media| {
        if media.url.starts_with("https://") || media.url.starts_with("http://") {
            return;
        }

        if media.url.starts_with('.') {
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

            if !old.starts_with('/') {
                url += "/";
            }

            url += old;
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
    match (&embed.canonical, &embed.url) {
        (Some(canonical), Some(url)) if canonical == url => {
            embed.canonical = None;
        }
        _ => {}
    }

    // redundant description
    match (&embed.title, &embed.description) {
        (Some(title), Some(description)) if title == description => {
            embed.description = None;
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

    // remove empty fields
    embed.fields.retain(|f| !EmbedField::is_empty(f));

    if let Some(ref img) = embed.img {
        match (img.width, img.height) {
            // if there is a tiny main image, relegate it down to a thumbnail
            (Some(w), Some(h)) if w <= 320 && h <= 320 => {
                embed.thumb = std::mem::take(&mut embed.img);

                if embed.ty == EmbedType::Img {
                    embed.ty = EmbedType::Link;
                }
            }
            _ => {}
        }
    }

    // Avoid alt-text that's the same as the description
    if embed.description.is_some() {
        // NOTE: SmolStr uses an Arc internally, so cloning is cheap
        let desc = embed.description.clone();

        embed.visit_media_mut(|media| {
            if media.alt == desc {
                media.alt = None;
            }
        });
    }

    super::embed::determine_embed_type(embed);
}
