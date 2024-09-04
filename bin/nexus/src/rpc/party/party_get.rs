use futures::TryStreamExt;

use crate::{prelude::*, rpc::SearchMode, util::encrypted_asset::encrypt_snowflake_opt};

use sdk::models::*;

pub async fn get_party(state: ServerState, auth: Authorization, party_id: PartyId) -> Result<Party, Error> {
    let db = state.db.read.get().await?;

    get_party_inner(state, &db, auth.user_id(), party_id).await
}

pub async fn get_party_inner(
    state: ServerState,
    db: &db::Client,
    user_id: UserId,
    party_id: PartyId,
) -> Result<Party, Error> {
    #[rustfmt::skip]
    let row = db.query_one2(schema::sql! {
        SELECT
            Party.Name AS @_,
            Party.Flags AS @_,
            Party.OwnerId AS @_,
            Party.AvatarId AS @_,
            Party.BannerId AS @_,
            Party.Description AS @_,
            Party.DefaultRoom AS @_,
            PartyMembers.Position AS @_
        FROM Party LEFT JOIN PartyMembers ON PartyMembers.PartyId = Party.Id
        WHERE Party.Id = #{&party_id as Party::Id}
        AND PartyMembers.UserId = #{&user_id as Users::Id}
        AND Party.DeletedAt IS NULL
    }).await?;

    let mut party = Party {
        partial: PartialParty {
            id: party_id,
            name: row.party_name()?,
            description: row.party_description()?,
        },
        flags: row.party_flags()?,
        avatar: encrypt_snowflake_opt(&state, row.party_avatar_id()?),
        banner: Nullable::Undefined,
        default_room: row.party_default_room()?,
        position: row.party_members_position()?,
        owner: row.party_owner_id()?,
        roles: ThinVec::new(),
        emotes: ThinVec::new(),
        folders: ThinVec::new(),
    };

    // these fields are only provided to joined members
    if party.position.is_some() {
        party.banner = encrypt_snowflake_opt(&state, row.party_banner_id()?).into();

        (party.roles, party.emotes) = tokio::try_join!(
            async {
                super::roles::get_roles::get_roles_raw(db, &state, SearchMode::Single(party_id))
                    .await?
                    .try_collect::<ThinVec<_>>()
                    .await
            },
            async {
                super::party_emotes::get_custom_emotes_raw(db, SearchMode::Single(party_id))
                    .await?
                    .map_ok(Emote::Custom)
                    .try_collect::<ThinVec<_>>()
                    .await
            }
        )?;
    }

    Ok(party)
}
