use futures::{StreamExt, TryStreamExt};

use schema::Snowflake;

use crate::{
    ctrl::{auth::Authorization, Error, SearchMode},
    ServerState,
};

use models::*;

pub async fn get_party(state: ServerState, auth: Authorization, party_id: Snowflake) -> Result<Party, Error> {
    let db = state.db.read.get().await?;

    let row = db
        .query_opt_cached_typed(
            || {
                use schema::*;
                use thorn::*;

                Query::select()
                    .cols(&[Party::Name, Party::OwnerId, Party::AvatarId, Party::Description])
                    .col(PartyMember::SortOrder)
                    .and_where(Party::Id.equals(Var::of(Party::Id)))
                    .from(Party::left_join_table::<PartyMember>().on(PartyMember::PartyId.equals(Party::Id)))
                    .and_where(PartyMember::UserId.equals(Var::of(Users::Id)))
                    .and_where(Party::DeletedAt.is_null())
            },
            &[&party_id, &auth.user_id],
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
            icon_id: row.try_get(2)?,
            sort_order: row.try_get::<_, i16>(4)? as u16,
        },
    };

    let roles = async {
        super::roles::get_roles_raw(&db, SearchMode::Single(party_id))
            .await?
            .try_collect::<Vec<_>>()
            .await
    };

    let emotes = async {
        super::emotes::get_custom_emotes_raw(&db, SearchMode::Single(party_id))
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
