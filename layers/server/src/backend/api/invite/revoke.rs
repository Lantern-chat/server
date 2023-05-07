use db::pg::error::SqlState;
use futures::{future::Either, FutureExt, TryFutureExt};

use schema::{Snowflake, SnowflakeExt};
use smol_str::SmolStr;

use crate::{Authorization, Error, ServerState};
use sdk::models::Permissions;

pub async fn revoke_invite(state: ServerState, auth: Authorization, code: SmolStr) -> Result<(), Error> {
    let maybe_id = crate::backend::util::encrypted_asset::decrypt_snowflake(&state, &code);

    #[rustfmt::skip]
    state.db.write.get().await?.execute2(schema::sql! {
        tables! {
            struct Perms {
                InviteId: Invite::Id,
                Allowed: Type::BOOL,
            }
        };

        WITH Perms AS (
            SELECT
                Invite.Id AS Perms.InviteId,
                // if this is the user that created the invite
                Invite.UserId = PartyMembers.UserId OR
                // or they have the correct party permissions
                let perms = Permissions::MANAGE_PARTY.to_i64();
                (PartyMembers.Permissions1 & {perms[0]} = {perms[0]} AND
                 PartyMembers.Permissions2 & {perms[1]} = {perms[1]}) AS Perms.Allowed
            FROM PartyMembers INNER JOIN Invite ON PartyMembers.PartyId = Invite.PartyId
            WHERE (Invite.Id = #{&maybe_id as Invite::Id}
            OR Invite.Vanity = #{&code as Invite::Vanity})
            AND PartyMembers.UserId = #{&auth.user_id as Users::Id}
        )
        UPDATE Invite SET (Uses, Expires) = (0, NOW())
        FROM Perms WHERE Invite.Id = Perms.InviteId AND Perms.Allowed IS TRUE
    }).await?;

    Ok(())
}
