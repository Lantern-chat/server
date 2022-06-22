use std::ops::Range;

section! {
    #[serde(default)]
    pub struct Message {
        pub max_newlines: usize                 = 80,

        #[serde(with = "super::util::range")]
        pub message_len: Range<usize>           = 1..2500,

        #[serde(with = "super::util::range")]
        pub premium_message_len: Range<usize>   = 1..5000,
    }
}
