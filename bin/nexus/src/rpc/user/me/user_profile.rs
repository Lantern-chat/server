use crate::prelude::*;

use sdk::models::*;

use sdk::api::commands::all::UpdateUserProfile;

pub async fn patch_user_profile(
    state: ServerState,
    auth: Authorization,
    cmd: &Archived<UpdateUserProfile>,
) -> Result<UserProfile, Error> {
    let profile = &cmd.body;

    crate::internal::user_profile::patch_profile(
        state,
        auth.user_id(),
        None,
        crate::internal::user_profile::PatchProfile {
            bits: profile.bits.to_native_truncate(),
            extra: profile.extra.to_native_truncate(),
            nick: profile.nick.as_ref().map(|s| s.as_str()),
            status: profile.status.as_ref().map(|s| s.as_str()),
            bio: profile.bio.as_ref().map(|s| s.as_str()),
            avatar: profile.avatar.map(Into::into),
            banner: profile.avatar.map(Into::into),
        },
    )
    .await
}
