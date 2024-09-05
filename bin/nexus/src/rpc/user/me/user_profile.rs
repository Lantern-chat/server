use crate::prelude::*;

use sdk::models::*;

use sdk::api::commands::all::UpdateUserProfile;

pub async fn patch_user_profile(
    state: ServerState,
    auth: Authorization,
    cmd: &Archived<UpdateUserProfile>,
) -> Result<UserProfile, Error> {
    crate::internal::user_profile::patch_profile(
        state,
        auth.user_id(),
        None,
        (&cmd.body).into(), //
    )
    .await
}
