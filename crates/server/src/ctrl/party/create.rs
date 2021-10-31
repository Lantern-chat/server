use std::time::SystemTime;

use schema::SnowflakeExt;

use models::*;
use smol_str::SmolStr;

use crate::{
    ctrl::{auth::Authorization, Error},
    ServerState,
};

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
    if !state.config.partyname_len.contains(&form.name.len()) {
        return Err(Error::InvalidName);
    }

    let now = SystemTime::now();

    let party_id = Snowflake::at(now);
    let room_id = Snowflake::at(now);

    let default_role = Role {
        id: party_id,
        party_id,
        avatar: None,
        name: None,
        permissions: Permission::default(),
        color: None,
        sort: 0,
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
        sort_order: 0,
    };

    let mut db = state.db.write.get().await?;
    let t = db.transaction().await?;

    // insert party first to avoid foreign key issues
    t.execute_cached_typed(
        || insert_party(),
        &[&party.id, &party.name, &party.description, &party.owner],
    )
    .await?;

    futures::future::try_join3(
        t.execute_cached_typed(|| insert_member(), &[&party.id, &auth.user_id]),
        t.execute_cached_typed(
            || insert_role(),
            &[
                &default_role.id,
                &party.id,
                &(default_role.permissions.pack() as i64),
            ],
        ),
        t.execute_cached_typed(
            || insert_room(),
            &[&room_id, &party.id, &"general", &0i16, &RoomFlags::DEFAULT.bits()],
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
        .cols(&[Party::Id, Party::Name, Party::Description, Party::OwnerId])
        .values(vec![
            Var::of(Party::Id),
            Var::of(Party::Name),
            Var::of(Party::Description),
            Var::of(Party::OwnerId),
        ])
}

fn insert_member() -> impl AnyQuery {
    use schema::*;

    Query::insert()
        .into::<PartyMember>()
        .cols(&[PartyMember::PartyId, PartyMember::UserId])
        .values(vec![Var::of(Party::Id), Var::of(Users::Id)])
}

// NOTE: Does not set sort order manually, defaults to 0
fn insert_role() -> impl AnyQuery {
    use schema::*;

    Query::insert()
        .into::<Roles>()
        .cols(&[Roles::Id, Roles::PartyId, Roles::Permissions])
        .values(vec![
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
            Rooms::SortOrder,
            Rooms::Flags,
        ])
        .values(vec![
            Var::of(Rooms::Id),
            Var::of(Party::Id),
            Var::of(Rooms::Name),
            Var::of(Rooms::SortOrder),
            Var::of(Rooms::Flags),
        ])
}
