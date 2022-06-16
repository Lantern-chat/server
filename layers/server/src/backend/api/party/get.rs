use futures::{StreamExt, TryStreamExt};

use schema::Snowflake;

use crate::{
    backend::{
        api::{auth::Authorization, SearchMode},
        util::encrypted_asset::encrypt_snowflake_opt,
    },
    Error, ServerState,
};

use sdk::models::*;
pub async fn get_party(state: ServerState, auth: Authorization, party_id: Snowflake) -> Result<Party, Error> {
    let db = state.db.read.get().await?;

    get_party_inner(state, &db, auth.user_id, party_id).await
}

pub async fn get_party_inner(
    state: ServerState,
    db: &db::pool::Client,
    user_id: Snowflake,
    party_id: Snowflake,
) -> Result<Party, Error> {
    let row = db
        .query_opt_cached_typed(
            || {
                use schema::*;
                use thorn::*;

                Query::select()
                    .cols(&[
                        /*0*/ Party::Name,
                        /*1*/ Party::OwnerId,
                        /*2*/ Party::AvatarId,
                        /*3*/ Party::Description,
                        /*4*/ Party::DefaultRoom,
                    ])
                    .col(/*5*/ PartyMember::Position)
                    .and_where(Party::Id.equals(Var::of(Party::Id)))
                    .from(Party::left_join_table::<PartyMember>().on(PartyMember::PartyId.equals(Party::Id)))
                    .and_where(PartyMember::UserId.equals(Var::of(Users::Id)))
                    .and_where(Party::DeletedAt.is_null())
            },
            &[&party_id, &user_id],
        )
        .await?;

    let mut party = match row {
        None => return Err(Error::NotFound),
        Some(row) => Party {
            partial: PartialParty {
                id: party_id,
                name: row.try_get(0)?,
                description: row.try_get(3)?,
            },
            owner: row.try_get(1)?,
            security: SecurityFlags::empty(),
            roles: Vec::new(),
            emotes: Vec::new(),
            avatar: encrypt_snowflake_opt(&state, row.try_get(2)?),
            position: row.try_get(5)?,
            default_room: row.try_get(4)?,
        },
    };

    let roles = async {
        super::roles::get_roles_raw(db, &state, SearchMode::Single(party_id))
            .await?
            .try_collect::<Vec<_>>()
            .await
    };

    let emotes = async {
        super::emotes::get_custom_emotes_raw(db, SearchMode::Single(party_id))
            .await?
            .map_ok(Emote::Custom)
            .try_collect::<Vec<_>>()
            .await
    };

    let (roles, emotes) = futures::future::join(roles, emotes).await;

    party.roles = roles?;
    party.emotes = emotes?;

    Ok(party)
}
