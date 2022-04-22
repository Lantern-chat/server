use std::ops::Range;

#[derive(Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct Party {
    #[serde(with = "super::util::range")]
    pub partyname_len: Range<usize>,

    #[serde(with = "super::util::range")]
    pub roomname_len: Range<usize>,
}

impl Default for Party {
    fn default() -> Party {
        Party {
            partyname_len: 3..64,
            roomname_len: 3..64,
        }
    }
}
