use std::ops::Range;

section! {
    #[serde(default)]
    pub struct Party {
        #[serde(with = "super::util::range")]
        pub partyname_len: Range<usize>     = 3..64,

        #[serde(with = "super::util::range")]
        pub roomname_len: Range<usize>      = 3..64,
    }
}
