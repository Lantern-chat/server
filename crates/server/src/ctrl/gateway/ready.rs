use futures::{StreamExt, TryStreamExt};
use hashbrown::{hash_map::Entry, HashMap};

use db::Snowflake;

use crate::{
    ctrl::{auth::Authorization, Error},
    ServerState,
};

struct Associated<T> {
    pub party_id: Snowflake,
    pub member: T,
}

pub async fn ready(
    state: ServerState,
    conn_id: Snowflake,
    auth: Authorization,
) -> Result<models::ReadyEvent, Error> {
    use models::*;

    let db = &state.db.read;

    let user = async {
        let row = db
            .query_one_cached_typed(|| select_user(), &[&auth.user_id])
            .await?;

        Ok::<_, Error>(User {
            id: auth.user_id,
            username: row.try_get(0)?,
            descriminator: row.try_get(1)?,
            flags: UserFlags::from_bits_truncate(row.try_get(2)?),
            email: Some(row.try_get(3)?),
            avatar_id: row.try_get(4)?,
            status: row.try_get(5)?,
            bio: row.try_get(6)?,
            preferences: {
                let value: Option<serde_json::Value> = row.try_get(7)?;

                match value {
                    None => None,
                    Some(v) => Some(serde_json::from_value(v)?),
                }
            },
        })
    };

    let parties = async {
        let rows = db
            .query_stream_cached_typed(|| select_parties(), &[&auth.user_id])
            .await?;

        let parties_stream = rows.map(|row| match row {
            Err(e) => Err(Error::from(e)),
            Ok(row) => Ok(Party {
                partial: PartialParty {
                    id: row.try_get(0)?,
                    name: row.try_get(2)?,
                    description: None,
                },
                owner: row.try_get(1)?,
                security: SecurityFlags::empty(),
                roles: Vec::new(),
                emotes: Vec::new(),
                members: Vec::new(),
            }),
        });

        let mut parties = HashMap::new();
        let mut ids = Vec::new();

        futures::pin_mut!(parties_stream);
        while let Some(res) = parties_stream.next().await {
            let party = res?;

            ids.push(party.id);
            parties.insert(party.id, party);
        }

        let roles = async {
            let rows = db
                .query_stream_cached_typed(|| select_roles(), &[&ids])
                .await?;

            rows.map(|row| match row {
                Err(e) => Err(Error::from(e)),
                Ok(row) => {
                    let permissions = Permission::unpack(row.try_get::<_, i64>(3)? as u64);
                    let party_id = row.try_get(1)?;

                    Ok(Associated {
                        party_id,
                        member: Role {
                            id: row.try_get(0)?,
                            name: row.try_get(2)?,
                            permissions,
                            admin: false,
                            color: row.try_get::<_, i32>(4)? as u32,
                            mentionable: false,
                        },
                    })
                }
            })
            .try_collect::<Vec<_>>()
            .await
        };

        let emotes = async {
            // todo: stuff
            Ok::<Vec<Associated<Emote>>, Error>(Vec::new())
        };

        let members = async {
            let rows = db
                .query_stream_cached_typed(|| select_members(), &[&ids])
                .await?;

            let mut members = HashMap::<(Snowflake, Snowflake), Associated<PartyMember>>::new();

            futures::pin_mut!(rows);
            while let Some(row) = rows.next().await {
                let row = row?;

                let party_id = row.try_get(0)?;
                let user_id = row.try_get(2)?;

                match members.entry((party_id, user_id)) {
                    Entry::Occupied(mut m) => m.get_mut().member.roles.push(row.try_get(6)?),
                    Entry::Vacant(v) => {
                        v.insert(Associated {
                            party_id,
                            member: PartyMember {
                                user: Some(User {
                                    id: user_id,
                                    username: row.try_get(3)?,
                                    descriminator: row.try_get(4)?,
                                    flags: UserFlags::from_bits_truncate(row.try_get(5)?)
                                        .publicize(),
                                    email: None,
                                    preferences: None,
                                    status: None,
                                    bio: None,
                                    avatar_id: None,
                                }),
                                nick: row.try_get(1)?,
                                roles: vec![row.try_get(6)?],
                            },
                        });
                    }
                }
            }

            // todo: stuff
            Ok::<Vec<Associated<PartyMember>>, Error>(members.into_iter().map(|(_, v)| v).collect())
        };

        let (roles, emotes, members) = futures::future::join3(roles, emotes, members).await;
        let (roles, emotes, members) = (roles?, emotes?, members?);

        for role in roles {
            if let Some(party) = parties.get_mut(&role.party_id) {
                party.roles.push(role.member);
            }
        }

        for member in members {
            if let Some(party) = parties.get_mut(&member.party_id) {
                party.members.push(member.member);
            }
        }

        for emote in emotes {
            if let Some(party) = parties.get_mut(&emote.party_id) {
                party.emotes.push(emote.member);
            }
        }

        Ok::<_, Error>(parties.into_iter().map(|(_, v)| v).collect())
    };

    let (user, parties) = futures::future::join(user, parties).await;

    Ok(ReadyEvent {
        user: user?,
        dms: Vec::new(),
        parties: parties?,
        session: conn_id,
    })
}

use thorn::*;

fn select_user() -> impl AnyQuery {
    use db::schema::*;

    Query::select()
        .from_table::<Users>()
        .and_where(Users::Id.equals(Var::of(Users::Id)))
        .cols(&[
            Users::Username,      // 0
            Users::Discriminator, // 1
            Users::Flags,         // 2
            Users::Email,         // 3
            Users::AvatarId,      // 4
            Users::CustomStatus,  // 5
            Users::Biography,     // 6
            Users::Preferences,   // 7
        ])
}

fn select_parties() -> impl AnyQuery {
    use db::schema::*;

    Query::select()
        .cols(&[Party::Id, Party::OwnerId, Party::Name])
        .from(Party::left_join_table::<PartyMember>().on(PartyMember::PartyId.equals(Party::Id)))
        .and_where(PartyMember::UserId.equals(Var::of(Users::Id)))
}

fn select_roles() -> impl AnyQuery {
    use db::schema::*;

    Query::select()
        .from_table::<Roles>()
        .cols(&[
            Roles::Id,
            Roles::PartyId,
            Roles::Name,
            Roles::Permissions,
            Roles::Color,
        ])
        .and_where(Roles::PartyId.equals(Builtin::any(Var::of(Type::INT8_ARRAY))))
}

fn select_members() -> impl AnyQuery {
    use db::schema::*;

    Query::select()
        .and_where(PartyMember::PartyId.equals(Builtin::any(Var::of(Type::INT8_ARRAY))))
        .cols(&[PartyMember::PartyId, PartyMember::Nickname])
        .cols(&[
            Users::Id,
            Users::Username,
            Users::Discriminator,
            Users::Flags,
        ])
        .col(RoleMembers::RoleId)
        .from(
            RoleMembers::right_join(
                Users::left_join_table::<PartyMember>().on(Users::Id.equals(PartyMember::UserId)),
            )
            .on(RoleMembers::UserId.equals(Users::Id)),
        )
}

/*
fn select_members_old() -> impl AnyQuery {
    use db::schema::*;

    Query::select()
        .and_where(PartyMember::PartyId.equals(Builtin::any(Var::of(Type::INT8_ARRAY))))
        .cols(&[PartyMember::PartyId, PartyMember::Nickname])
        .cols(&[
            Users::Id,
            Users::Username,
            Users::Discriminator,
            Users::Flags,
        ])
        .expr(
            Query::select()
                .from_table::<RoleMembers>()
                .expr(Builtin::array_agg(RoleMembers::RoleId))
                .and_where(RoleMembers::UserId.equals(Users::Id))
                .as_value(),
        )
        .from(Users::left_join_table::<PartyMember>().on(Users::Id.equals(PartyMember::UserId)))
}

fn select_emotes() -> impl AnyQuery {
    use db::schema::*;

    Query::select()
}
*/
