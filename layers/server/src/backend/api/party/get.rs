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
    mod party_query {
        pub use schema::*;
        use thorn::indexed_columns;
        pub use thorn::*;

        indexed_columns! {
            pub enum PartyColumns {
                Party::Name,
                Party::OwnerId,
                Party::AvatarId,
                Party::BannerId,
                Party::Description,
                Party::DefaultRoom,
            }

            pub enum PartyMemberColumns continue PartyColumns {
                PartyMember::Position,
            }
        }
    }

    let row = db
        .query_opt_cached_typed(
            || {
                use party_query::*;

                Query::select()
                    .cols(PartyColumns::default())
                    .cols(PartyMemberColumns::default())
                    .and_where(Party::Id.equals(Var::of(Party::Id)))
                    .from(Party::left_join_table::<PartyMember>().on(PartyMember::PartyId.equals(Party::Id)))
                    .and_where(PartyMember::UserId.equals(Var::of(Users::Id)))
                    .and_where(Party::DeletedAt.is_null())
            },
            &[&party_id, &user_id],
        )
        .await?;

    use party_query::{PartyColumns, PartyMemberColumns};

    let Some(row) = row else {
        return Err(Error::NotFound);
    };

    let mut party = Party {
        partial: PartialParty {
            id: party_id,
            name: row.try_get(PartyColumns::name())?,
            description: row.try_get(PartyColumns::description())?,
        },
        avatar: encrypt_snowflake_opt(&state, row.try_get(PartyColumns::avatar_id())?),
        banner: Nullable::Undefined,
        default_room: row.try_get(PartyColumns::default_room())?,
        position: row.try_get(PartyMemberColumns::position())?,
        security: SecurityFlags::empty(),
        owner: row.try_get(PartyColumns::owner_id())?,
        roles: Vec::new(),
        emotes: Vec::new(),
        pin_folders: Vec::new(),
    };

    // these fields are only provided to joined members
    if party.position.is_some() {
        party.banner = encrypt_snowflake_opt(&state, row.try_get(PartyColumns::banner_id())?).into();

        (party.roles, party.emotes) = tokio::try_join!(
            async {
                super::roles::get_roles_raw(db, &state, SearchMode::Single(party_id))
                    .await?
                    .try_collect::<Vec<_>>()
                    .await
            },
            async {
                super::emotes::get_custom_emotes_raw(db, SearchMode::Single(party_id))
                    .await?
                    .map_ok(Emote::Custom)
                    .try_collect::<Vec<_>>()
                    .await
            }
        )?;
    }

    Ok(party)
}
