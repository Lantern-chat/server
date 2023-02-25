//! Brand colors
//!
//! I hate this. All of it. Just embed the correct color you damn websites.

const YOUTUBE: u32 = 0xFF0000;
const TWITTER: u32 = 0x1DA1F2;
const REDDIT: u32 = 0xFF4500;
const DISCORD: u32 = 0x5865F2;

pub static BRAND_COLORS: phf::Map<&'static str, u32> = phf::phf_map! {
    // YouTube (more complex domains handled below)
    // https://www.netify.ai/resources/applications/youtube
    "googlevideo.com" => YOUTUBE,
    "gvt1.com" => YOUTUBE,
    "video.google.com" => YOUTUBE,
    "video.l.google.com" => YOUTUBE,
    "youtu.be" => YOUTUBE,
    "youtube.com" => YOUTUBE,
    "youtube-nocookie.com" => YOUTUBE,
    "youtube-ui.l.google.com" => YOUTUBE,
    "youtubeeducation.com" => YOUTUBE,
    "youtubeembeddedplayer.googleapis.com" => YOUTUBE,
    "youtubei.googleapis.com" => YOUTUBE,
    "youtube.googleapis.com" => YOUTUBE,
    "youtubekids.com" => YOUTUBE,
    "yt-video-upload.l.google.com" => YOUTUBE,
    "yt.be" => YOUTUBE,
    "yt3.ggpht.com" => YOUTUBE,
    "ytimg.com" => YOUTUBE,
    "ytimg.l.google.com" => YOUTUBE,
    "ytkids.app.goo.gl" => YOUTUBE,

    // Twitter
    // https://www.netify.ai/resources/applications/twitter
    "t.co" => TWITTER,
    "tweetdeck.com" => TWITTER,
    "twimg.com" => TWITTER,
    "twitpic.com" => TWITTER,
    "twitter.co" => TWITTER,
    "twitter.com" => TWITTER,
    "twitterinc.com" => TWITTER,
    "twitteroauth.com" => TWITTER,
    "twitterstat.us" => TWITTER,
    "twttr.com" => TWITTER,

    // Reddit
    // https://www.netify.ai/resources/applications/reddit
    // https://securitytrails.com/list/apex_domain/www.reddit.com
    "redd.it" => REDDIT,
    "reddit.com" => REDDIT,
    "redditblog.com" => REDDIT,
    "redditinc.com" => REDDIT,
    "redditmail.com" => REDDIT,
    "redditmedia.com" => REDDIT,
    "redditstatic.com" => REDDIT,
    "redditstatus.com" => REDDIT,
    "uninews.www.reddit.com" => REDDIT,
    "uninews.reddit.com" => REDDIT,
    "mobile.www.reddit.com" => REDDIT,
    "mobile.reddit.com" => REDDIT,
    "jie.www.reddit.com" => REDDIT,
    "jie.reddit.com" => REDDIT,
    "old.www.reddit.com" => REDDIT,
    "old.reddit.com" => REDDIT,
    "np.www.reddit.com" => REDDIT,
    "np.reddit.com" => REDDIT,
    "node.www.reddit.com" => REDDIT,
    "node.reddit.com" => REDDIT,

    // Discord
    // https://www.netify.ai/resources/applications/discord
    // https://subdomainfinder.c99.nl/scans/2020-05-03/discord.com
    "discord.com" => DISCORD,
    "discord.gg" => DISCORD,
    "discord.media" => DISCORD,
    "discordapp.com" => DISCORD,
    "discordapp.net" => DISCORD,
    "cdn.discordapp.com" => DISCORD,
    "cdn.discordapp.net" => DISCORD,
    "discordstatus.com" => DISCORD,
    "canary.discord.com" => DISCORD,
    "ptb.discord.com" => DISCORD,
    "blog.discord.com" => DISCORD,
    "www.discord.com" => DISCORD,
    "printer.discord.com" => DISCORD,
    "safety.discord.com" => DISCORD,
    "status.discord.com" => DISCORD,
};

pub fn get_brand_color(mut domain: &str) -> Option<u32> {
    while let Some(simplified) = domain.strip_prefix("www.") {
        domain = simplified;
    }

    if let Some(bc) = BRAND_COLORS.get(domain) {
        return Some(*bc);
    }

    // twitter has over 1000 subdomains
    if domain.ends_with("twitter.com") {
        return Some(TWITTER);
    }

    'yt: {
        // parse exotic youtube domains to verify
        if domain.starts_with("youtu") {
            let mut chunks = domain.split('.');

            let base = chunks.next()?;
            let tld = chunks.next()?;

            if !matches!(base, "youtube" | "youtu") {
                break 'yt;
            }

            if !matches!(tld, "com" | "co" | _ if YT_CODES.contains(&tld)) {
                break 'yt;
            }

            if let Some(lc) = chunks.next() {
                if !YT_CODES.contains(&lc) {
                    break 'yt;
                }
            }

            // likely a fake domain, skip
            if chunks.next().is_some() {
                break 'yt;
            }

            return Some(YOUTUBE);
        }
    }

    None
}

static YT_CODES: &[&str] = &[
    "ar", "au", "az", "bd", "bh", "bo", "br", "by", "co", "cr", "cz", "de", "dk", "do", "ec", "ee", "eg", "es",
    "fi", "fr", "ge", "gh", "gr", "gt", "hk", "hn", "hr", "hu", "ie", "in", "iq", "is", "it", "jm", "jo", "jp",
    "kr", "kw", "kz", "la", "lb", "lk", "lt", "lu", "lv", "ly", "ma", "md", "me", "mk", "mn", "mt", "mx", "my",
    "ng", "ni", "nl", "no", "om", "pa", "pe", "ph", "pk", "pl", "pr", "pt", "py", "qa", "ro", "rs", "ru", "sa",
    "se", "sg", "si", "sk", "sn", "soy", "sv", "tn", "tr", "tv", "tw", "ua", "ug", "uy", "ve", "vn",
];
