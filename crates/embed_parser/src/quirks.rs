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
    if let Some(ref img) = embed.img {
        if let Some(ref mime) = img.mime {
            if !mime.starts_with("image") {
                embed.img = None;
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
}

pub static AVOID_OEMBED: phf::Set<&'static str> = phf::phf_set!("fxtwitter.com");

// TODO: Add Lantern's user-agent to vxtwitter main
pub static USER_AGENTS: phf::Map<&'static str, &'static str> = phf::phf_map! {
    "vxtwitter.com" => "test",
};