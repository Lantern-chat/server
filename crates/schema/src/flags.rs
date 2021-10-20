bitflags::bitflags! {
    pub struct FileFlags: i16 {
        const PARTIAL = 1 << 0;
        const COMPLETE = 1 << 1;
    }
}

bitflags::bitflags! {
    pub struct MemberFlags: i16 {
        const BANNED = 1 << 0;
        const SUPPORTER = 1 << 1;
    }
}
