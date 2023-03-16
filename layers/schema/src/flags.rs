bitflags::bitflags! {
    pub struct FileFlags: i16 {
        /// File upload is still in progress
        const PARTIAL = 1 << 0;
        /// Fill upload is complete
        const COMPLETE = 1 << 1;
    }

    pub struct MemberFlags: i16 {
        const BANNED = 1 << 0;
        const SUPPORTER = 1 << 1;
    }

    pub struct AttachmentFlags: i16 {
        /// The attachment still exists, but it has been removed from the parent message
        ///
        /// This is required to preserve existing links, but not include it in queries.
        const ORPHANED = 1 << 0;
    }

    pub struct RoomMemberFlags: i32 {
        const MUTED = 1 << 0;
    }
}
