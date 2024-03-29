bitflags::bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub struct FileFlags: i16 {
        /// File upload is still in progress
        const PARTIAL = 1 << 0;
        /// Fill upload is complete
        const COMPLETE = 1 << 1;
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub struct MemberFlags: i16 {
        const BANNED = 1 << 0;
        const SUPPORTER = 1 << 1;
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub struct AttachmentFlags: i16 {
        /// The attachment still exists, but it has been removed from the parent message
        ///
        /// This is required to preserve existing links, but not include it in queries.
        const ORPHANED = 1 << 0;
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub struct RoomMemberFlags: i32 {
        const MUTED = 1 << 0;
    }
}
