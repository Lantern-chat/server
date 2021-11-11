use std::collections::HashMap;

pub enum EmbedType {
    XML,
    JSON,
}

pub struct OEmbed<'a> {
    pub ty: EmbedType,
    pub link: &'a str,
}

pub struct Discovery<'a> {
    pub og: HashMap<&'a str, &'a str>,
    pub oembed: Option<OEmbed<'a>>,
    pub theme_color: &'a str,
}
