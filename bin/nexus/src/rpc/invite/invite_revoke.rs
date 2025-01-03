use smol_str::SmolStr;

use crate::prelude::*;
use sdk::models::Permissions;

pub async fn revoke_invite(state: ServerState, auth: Authorization, code: SmolStr) -> Result<(), Error> {
    let maybe_id = crate::util::encrypted_asset::decrypt_snowflake(&state, &code);

    #[rustfmt::skip]
    state.db.write.get().await?.execute2(schema::sql! {
        const_assert!(!Columns::IS_DYNAMIC);

        struct Perms {
            InviteId: Invite::Id,
            Allowed: Type::BOOL,
        }

        WITH Perms AS (
            SELECT
                Invite.Id AS Perms.InviteId,
                // if this is the user that created the invite
                Invite.UserId = PartyMembers.UserId OR
                // or they have the correct party permissions
                const PERMS: [i64; 2] = Permissions::MANAGE_PARTY.to_i64();
                (PartyMembers.Permissions1 & const {PERMS[0]} = const {PERMS[0]} AND
                 PartyMembers.Permissions2 & const {PERMS[1]} = const {PERMS[1]}) AS Perms.Allowed
            FROM PartyMembers INNER JOIN Invite ON PartyMembers.PartyId = Invite.PartyId
            WHERE (Invite.Id = #{&maybe_id as Invite::Id}
            OR Invite.Vanity = #{&code as Invite::Vanity})
            AND PartyMembers.UserId = #{auth.user_id_ref() as Users::Id}
        )
        UPDATE Invite SET (Uses, Expires) = (0, NOW())
        FROM Perms WHERE Invite.Id = Perms.InviteId AND Perms.Allowed IS TRUE
    }).await?;

    Ok(())
}
