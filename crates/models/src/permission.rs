bitflags::bitflags! {
    pub struct Permission: u32 {
        const SEND_MESSAGE = 1 << 0;
        const READ_MESSAGE = 1 << 1;
    }
}

serde_shims::impl_serde_for_bitflags!(Permission);
