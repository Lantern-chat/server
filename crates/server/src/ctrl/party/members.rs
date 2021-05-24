use hashbrown::hash_map::{Entry, HashMap};

use db::Snowflake;

use crate::{
    ctrl::{auth::AuthToken, Error},
    ServerState,
};

#[derive(Deserialize)]
pub struct LoginForm {
    pub email: String,
    pub password: String,
}

use futures::{Stream, StreamExt};

use models::{PartyMember, User, UserFlags};

pub fn get_members(
    state: ServerState,
    party_id: Snowflake,
) -> impl Stream<Item = Result<PartyMember, Error>> {
    async_stream::stream! {
        let rows = state.read_db().await
            .query_stream_cached_typed(|| select_members(), &[&party_id])
            .await;

        let rows = match rows {
            Err(e) => return yield Err(e.into()),
            Ok(rows) => rows,
        };

        let mut member = None;

        futures::pin_mut!(rows);
        loop {
            match rows.next().await {
                None => break,
                Some(Err(e)) => return yield Err(e.into()),
                Some(Ok(row)) => match parse_row(row, &mut member) {
                    Err(e) => return yield Err(e),
                    Ok(Some(m)) => yield Ok(m),
                    Ok(None) => {}
                }
            }
        }

        // cleanup last member
        if let Some(member) = member {
            yield Ok(member);
        }
    }
}

use thorn::*;

fn select_members() -> impl AnyQuery {
    use db::schema::*;

    Query::select()
        .and_where(PartyMember::PartyId.equals(Var::of(Party::Id)))
        .cols(&[PartyMember::Nickname])
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
        .order_by(Users::Id.ascending())
}

fn parse_row(
    row: db::Row,
    existing: &mut Option<PartyMember>,
) -> Result<Option<PartyMember>, Error> {
    let user_id = row.try_get(1)?;
    let role_id = row.try_get(5)?;

    match existing {
        Some(PartyMember {
            user: Some(ref user),
            ref mut roles,
            ..
        }) => {
            // fast path, existing member with same id
            if user.id == user_id {
                roles.push(role_id);
                return Ok(None);
            }
        }
        Some(_) => unreachable!(),
        None => {}
    }

    let previous = existing.take();

    *existing = Some(PartyMember {
        user: Some(User {
            id: user_id,
            username: row.try_get(2)?,
            discriminator: row.try_get(3)?,
            flags: UserFlags::from_bits_truncate(row.try_get(4)?).publicize(),
            email: None,
            preferences: None,
            status: None,
            bio: None,
            avatar_id: None,
        }),
        nick: row.try_get(0)?,
        roles: vec![role_id],
    });

    Ok(previous)
}