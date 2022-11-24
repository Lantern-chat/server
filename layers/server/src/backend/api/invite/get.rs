use smol_str::SmolStr;

use crate::{
    backend::util::encrypted_asset::{decrypt_snowflake, encrypt_snowflake},
    Error, ServerState,
};

use sdk::models::*;

pub async fn get_invite(state: &ServerState, code: SmolStr) -> Result<Invite, Error> {
    let id = decrypt_snowflake(state, &code);

    let db = state.db.read.get().await?;

    use q::*;

    let row = db
        .query_opt_cached_typed(
            || {
                use schema::*;
                use thorn::*;

                Query::select()
                    .cols(InviteColumns::default())
                    .cols(PartyColumns::default())
                    .from(Invite::left_join_table::<Party>().on(Party::Id.equals(Invite::PartyId)))
                    .and_where(
                        Invite::Id
                            .equals(Var::of(SNOWFLAKE))
                            .or(Invite::Vanity.equals(Var::of(Type::TEXT))),
                    )
            },
            &[&id, &code],
        )
        .await?;

    let Some(row) = row else { return Err(Error::NotFound); };

    let vanity: Option<SmolStr> = row.try_get(InviteColumns::Vanity as usize)?;

    Ok(Invite {
        code: match vanity {
            Some(vanity) => vanity,
            None => encrypt_snowflake(state, row.try_get(InviteColumns::Id as usize)?),
        },
        party: PartialParty {
            id: row.try_get(InviteColumns::PartyId as usize)?,
            name: row.try_get(PartyColumns::Name as usize)?,
            description: row.try_get(PartyColumns::Description as usize)?,
        },
        description: row.try_get(InviteColumns::Description as usize)?,
        inviter: None,
        expires: row.try_get(InviteColumns::Expires as usize)?,
        remaining: None,
    })
}
mod q {
    use schema::{Invite, Party};
    thorn::indexed_columns! {
        pub enum InviteColumns {
            Invite::Id,
            Invite::UserId,
            Invite::PartyId,
            Invite::Expires,
            Invite::Uses,
            Invite::Description,
            Invite::Vanity,
        }

        pub enum PartyColumns continue InviteColumns {
            Party::Name,
            Party::Description,
        }
    }
}
