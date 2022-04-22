use std::ops::Range;

#[derive(Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct Message {
    pub max_newlines: usize,

    #[serde(with = "super::util::range")]
    pub message_len: Range<usize>,

    #[serde(with = "super::util::range")]
    pub premium_message_len: Range<usize>,
}

impl Default for Message {
    fn default() -> Message {
        Message {
            max_newlines: 80,
            message_len: 1..2500,
            premium_message_len: 1..5000,
        }
    }
}
