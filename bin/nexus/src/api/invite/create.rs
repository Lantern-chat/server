use crate::prelude::*;
use schema::Snowflake;
use timestamp::Duration;

use sdk::api::commands::party::CreatePartyInviteBody;
use sdk::models::*;

// 100 years
const MAX_DURATION: u64 = 100 * 365 * 24 * 60 * 60 * 1000;

pub async fn create_invite(
    state: ServerState,
    auth: Authorization,
    party_id: Snowflake,
    body: CreatePartyInviteBody,
) -> Result<Invite, Error> {
    let duration = match body.duration {
        Some(ms) if ms < MAX_DURATION => Some(Duration::milliseconds(ms as i64)),
        None => None,
        _ => return Err(Error::BadRequest),
    };

    let id = state.sf.gen();
    let expires = duration.map(|dur| state.sf.resolve_timestamp(id).saturating_add(dur));
    let uses = body.max_uses.map(|u| u as i32);

    #[rustfmt::skip]
    let row = state.db.write.get().await?.query_one2(schema::sql! {
        tables! {
            struct Checked {
                Allowed: Type::BOOL,
                PartyName: Party::Name,
                PartyDesc: Party::Description,
            }

            struct Inserted {
                InviteId: Invite::Id,
            }
        };

        WITH Checked AS (
            SELECT
                Party.Name AS Checked.PartyName,
                Party.Description AS Checked.PartyDesc,

                let perms = Permissions::CREATE_INVITE.to_i64();
                (PartyMembers.Permissions1 & {perms[0]} = {perms[0]} AND
                 PartyMembers.Permissions2 & {perms[1]} = {perms[1]}) AS Checked.Allowed
            FROM PartyMembers INNER JOIN LiveParties AS Party ON Party.Id = PartyMembers.PartyId
            WHERE PartyMembers.UserId = #{auth.user_id_ref() as Users::Id}
            AND PartyMembers.PartyId = #{&party_id as Party::Id}
        ), Inserted AS (
            INSERT INTO Invite (Id, PartyId, UserId, Expires, Uses, MaxUses, Description) (
                SELECT
                    #{&id as Invite::Id},
                    #{&party_id as Party::Id},
                    #{auth.user_id_ref() as Users::Id},
                    #{&expires as Invite::Expires},
                    #{&uses as Invite::Uses},
                    #{&uses as Invite::MaxUses},
                    #{&body.description as Invite::Description}
                FROM Checked WHERE Checked.Allowed IS TRUE
            ) RETURNING Invite.Id AS Inserted.InviteId
        )
        SELECT
            Checked.PartyName AS @PartyName,
            Checked.PartyDesc AS @PartyDesc,
            Inserted.InviteId AS @InviteId
        FROM
            Checked LEFT JOIN Inserted ON TRUE
    }).await?;

    if row.invite_id::<Option<Snowflake>>()?.is_none() {
        return Err(Error::Unauthorized);
    }

    Ok(Invite {
        code: crate::util::encrypted_asset::encrypt_snowflake(&state, id),
        party: PartialParty {
            id: party_id,
            name: row.party_name()?,
            description: row.party_desc()?,
        },
        inviter: Some(auth.user_id()),
        description: body.description,
        expires,
        remaining: body.max_uses,
    })
}
