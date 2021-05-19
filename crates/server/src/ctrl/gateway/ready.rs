use db::Snowflake;

use crate::{
    ctrl::{auth::Authorization, Error},
    ServerState,
};

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
        // testing
        Ok::<_, Error>(Vec::new())
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
