use std::ops::Range;

section! {
    #[serde(default)]
    pub struct Party {
        #[serde(with = "super::util::range")]
        pub party_name_len: Range<usize>     = 3..64,

        #[serde(with = "super::util::range")]
        pub party_description_len: Range<usize> = 1..1024,

        #[serde(with = "super::util::range")]
        pub room_name_len: Range<usize>      = 3..64,

        #[serde(with = "super::util::range")]
        pub room_topic_len: Range<usize>    = 1..512,

        #[serde(with = "super::util::range")]
        pub role_name_len: Range<usize>     = 1..64,

        /// Max rooms that are not deleted at any given time
        pub max_active_rooms: u16   = 128,

        /// Max rooms total, including deleted rooms
        ///
        /// Parties that encounter this limit will not be able to create new rooms
        /// until they have contacted support to purge all the deleted rooms.
        ///
        /// This helps prevent room-spams
        pub max_rooms: u16        = 1024,
    }
}
