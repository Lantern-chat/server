use std::time::SystemTime;

use schema::SnowflakeExt;

use sdk::models::*;
use smol_str::SmolStr;

use crate::{Authorization, Error, ServerState};

#[derive(Debug, Clone, Deserialize)]
pub struct PartyCreateForm {
    name: SmolStr,

    #[serde(default)]
    description: Option<SmolStr>,

    #[serde(default)]
    security: SecurityFlags,
}

pub async fn create_party(
    state: ServerState,
    auth: Authorization,
    form: PartyCreateForm,
) -> Result<Party, Error> {
    if !state.config.party.partyname_len.contains(&form.name.len()) {
        return Err(Error::InvalidName);
    }

    let now = SystemTime::now();

    let party_id = Snowflake::at(now);
    let room_id = Snowflake::at(now);

    let default_role = Role {
        id: party_id,
        party_id,
        avatar: None,
        name: SmolStr::new_inline("@everyone"),
        permissions: Permission::default(),
        color: None,
        position: 0,
        flags: RoleFlags::default(),
    };

    let mut party = Party {
        partial: PartialParty {
            id: party_id,
            name: form.name,
            description: form.description,
        },
        owner: auth.user_id,
        security: form.security,
        roles: Vec::new(),
        emotes: Vec::new(),
        avatar: None,
        banner: Nullable::Null,
        position: 0,
        default_room: room_id,
    };

    let mut db = state.db.write.get().await?;
    let t = db.transaction().await?;

    // insert party first to avoid foreign key issues
    t.execute_cached_typed(
        || insert_party(),
        &[
            &party.id,
            &party.name,
            &party.description,
            &party.owner,
            &party.default_room,
        ],
    )
    .await?;

    let position = {
        let row = t
            .query_one_cached_typed(|| query_max_position(), &[&auth.user_id])
            .await?;

        match row.try_get::<_, Option<i16>>(0)? {
            Some(max_position) => max_position + 1,
            None => 0,
        }
    };

    futures::future::try_join3(
        t.execute_cached_typed(|| insert_member(), &[&party.id, &auth.user_id, &position]),
        t.execute_cached_typed(
            || insert_role(),
            &[
                &default_role.name,
                &default_role.id,
                &party.id,
                &(default_role.permissions.pack() as i64),
            ],
        ),
        t.execute_cached_typed(
            || insert_room(),
            &[&room_id, &party.id, &"general", &0i16, &RoomFlags::DEFAULT],
        ),
    )
    .await?;

    t.commit().await?;

    party.roles.push(default_role);

    Ok(party)
}

use thorn::*;

fn insert_party() -> impl AnyQuery {
    use schema::*;

    Query::insert()
        .into::<Party>()
        .cols(&[
            Party::Id,
            Party::Name,
            Party::Description,
            Party::OwnerId,
            Party::DefaultRoom,
        ])
        .values([
            Var::of(Party::Id),
            Var::of(Party::Name),
            Var::of(Party::Description),
            Var::of(Party::OwnerId),
            Var::of(Party::DefaultRoom),
        ])
}

fn query_max_position() -> impl AnyQuery {
    use schema::*;

    Query::select()
        .expr(Builtin::max(PartyMember::Position))
        .from_table::<PartyMember>()
        .and_where(PartyMember::UserId.equals(Var::of(Users::Id)))
}

fn insert_member() -> impl AnyQuery {
    use schema::*;

    Query::insert()
        .into::<PartyMember>()
        .cols(&[PartyMember::PartyId, PartyMember::UserId, PartyMember::Position])
        .values([
            Var::of(Party::Id),
            Var::of(Users::Id),
            Var::of(PartyMember::Position),
        ])
}

// NOTE: Does not set sort order manually, defaults to 0
fn insert_role() -> impl AnyQuery {
    use schema::*;

    Query::insert()
        .into::<Roles>()
        .cols(&[Roles::Name, Roles::Id, Roles::PartyId, Roles::Permissions])
        .values([
            Var::of(Roles::Name),
            Var::of(Roles::Id),
            Var::of(Roles::PartyId),
            Var::of(Roles::Permissions),
        ])
}

fn insert_room() -> impl AnyQuery {
    use schema::*;

    Query::insert()
        .into::<Rooms>()
        .cols(&[
            Rooms::Id,
            Rooms::PartyId,
            Rooms::Name,
            Rooms::Position,
            Rooms::Flags,
        ])
        .values([
            Var::of(Rooms::Id),
            Var::of(Party::Id),
            Var::of(Rooms::Name),
            Var::of(Rooms::Position),
            Var::of(Rooms::Flags),
        ])
}
