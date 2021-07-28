bitflags::bitflags! {
    pub struct FileFlags: i16 {
        const PARTIAL = 1 << 0;
        const COMPLETE = 1 << 1;
    }
}
