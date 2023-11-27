use std::ops::Range;

config::section! {
    #[serde(default)]
    pub struct Message {
        pub max_newlines: usize                 = 80,

        #[serde(with = "config::util::range")]
        pub message_len: Range<usize>           = 1..2500,

        #[serde(with = "config::util::range")]
        pub premium_message_len: Range<usize>   = 1..5000,

        /// Maximum number of links to generate embeds for in a message
        pub max_embeds: u8                      = 8,

        /// Maximum length of a string allowed to be used as a regex for premium message search
        pub max_regex_search_len: u16           = 128,
    }
}
