use smol_str::SmolStr;

use crate::{
    backend::util::encrypted_asset::{decrypt_snowflake, encrypt_snowflake},
    Authorization, Error, ServerState,
};

use sdk::models::*;

pub async fn get_invite(state: &ServerState, auth: Authorization, code: SmolStr) -> Result<Invite, Error> {
    let id = decrypt_snowflake(state, &code);

    #[rustfmt::skip]
    let Some(row) = state.db.read.get().await?.query_opt2(schema::sql! {
        SELECT
            PartyMembers.Permissions1 AS @Permissions1,
            PartyMembers.Permissions2 AS @Permissions2,
            Invite.Id           AS @_,
            Invite.UserId       AS @UserId,
            Invite.PartyId      AS @PartyId,
            Invite.Expires      AS @Expires,
            Invite.Uses         AS @Uses,
            Invite.Description  AS @_,
            Invite.Vanity       AS @Vanity,
            Party.Name          AS @_,
            Party.Description   AS @_
        FROM Invite INNER JOIN LiveParties AS Party ON Party.Id = Invite.PartyId
        LEFT JOIN PartyMembers ON PartyMembers.UserId = #{&auth.user_id as Users::Id}
        WHERE Invite.Id = #{&id as Invite::Id} OR Invite.Vanity = #{&code as Invite::Vanity}
    }).await? else {
        return Err(Error::NotFound);
    };

    let inviter: Snowflake = row.user_id()?;

    let can_view_metadata = match (row.permissions1()?, row.permissions2()?) {
        (Some(low), Some(high)) => {
            // the person that created the invite can always view it so long as they are a party member,
            // otherwise they need additional permissions
            inviter == auth.user_id || Permissions::from_i64(low, high).intersects(Permissions::MANAGE_PARTY)
        }
        _ => false,
    };

    Ok(Invite {
        party: PartialParty {
            id: row.party_id()?,
            name: row.party_name()?,
            description: row.party_description()?,
        },
        code: match row.vanity()? {
            Some(vanity) => vanity,
            None => encrypt_snowflake(state, row.invite_id()?),
        },
        description: row.invite_description()?,
        inviter: can_view_metadata.then_some(inviter),
        expires: row.expires()?,
        remaining: if can_view_metadata { row.uses::<Option<i16>>()?.map(|u| u as u16) } else { None },
    })
}
