use crate::prelude::*;

use sdk::models::*;

use sdk::api::commands::all::UpdateMemberProfile;

pub async fn patch_member_profile(
    state: ServerState,
    auth: Authorization,
    cmd: &Archived<UpdateMemberProfile>,
) -> Result<UserProfile, Error> {
    crate::internal::user_profile::patch_profile(
        state,
        auth.user_id(),
        Some(cmd.party_id.into()),
        (&cmd.body.profile).into(), //
    )
    .await
}
