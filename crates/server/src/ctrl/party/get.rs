use futures::{StreamExt, TryStreamExt};

use db::Snowflake;

use crate::{
    ctrl::{auth::Authorization, Error},
    ServerState,
};

use models::{Emote, PartialParty, Party, Permission, Role, SecurityFlags};

pub async fn get_party(
    state: ServerState,
    auth: Authorization,
    party_id: Snowflake,
) -> Result<Party, Error> {
    let row = state
        .db
        .read
        .query_opt_cached_typed(
            || {
                use db::schema::*;
                use thorn::*;

                Query::select()
                    .cols(&[
                        Party::DeletedAt,
                        Party::Name,
                        Party::OwnerId,
                        Party::IconId,
                        Party::Description,
                    ])
                    .and_where(Party::Id.equals(Var::of(Party::Id)))
                    .from(
                        Party::left_join_table::<PartyMember>()
                            .on(PartyMember::PartyId.equals(Party::Id)),
                    )
                    .and_where(PartyMember::UserId.equals(Var::of(Users::Id)))
            },
            &[&party_id, &auth.user_id],
        )
        .await?;

    let mut party = match row {
        None => return Err(Error::NotFound),
        Some(row) => {
            let deleted_at: Option<time::PrimitiveDateTime> = row.try_get(0)?;

            if deleted_at.is_some() {
                return Err(Error::NotFound);
            }

            Party {
                partial: PartialParty {
                    id: party_id,
                    name: row.try_get(1)?,
                    description: row.try_get(4)?,
                },
                owner: row.try_get(2)?,
                security: SecurityFlags::empty(),
                roles: Vec::new(),
                emotes: Vec::new(),
            }
        }
    };

    let roles = state
        .db
        .read
        .query_stream_cached_typed(
            || {
                use db::schema::*;
                use thorn::*;

                Query::select()
                    .from_table::<Roles>()
                    .cols(&[Roles::Id, Roles::Name, Roles::Permissions, Roles::Color])
                    .and_where(Roles::PartyId.equals(Var::of(Party::Id)))
            },
            &[&party_id],
        )
        .await?;

    futures::pin_mut!(roles);
    while let Some(row) = roles.next().await {
        let row = row?;

        party.roles.push(Role {
            id: row.try_get(0)?,
            name: row.try_get(1)?,
            admin: false,
            permissions: Permission::unpack(row.try_get::<_, i64>(2)? as u64),
            color: row.try_get::<_, i32>(3)? as u32,
            mentionable: false,
        });
    }

    Ok(party)
}
