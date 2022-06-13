use smol_str::SmolStr;

pub enum EmbedWorkItem {
    Generic { url: SmolStr },
    OEmbed { url: SmolStr },
}
