use sdk::api::commands::all::BannerAlign;

use crate::prelude::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AssetMode {
    Avatar,
    Banner(BannerAlign),
}

pub async fn maybe_add_asset(
    state: &ServerState,
    mode: AssetMode,
    user_id: UserId,
    file_id: Nullable<FileId>,
) -> Result<Nullable<FileId>, Error> {
    Ok(Nullable::Null)
}
