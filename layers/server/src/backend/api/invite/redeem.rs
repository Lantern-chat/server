use db::pg::error::SqlState;
use futures::{future::Either, FutureExt, TryFutureExt};

use schema::{Snowflake, SnowflakeExt};
use smol_str::SmolStr;

use crate::{Authorization, Error, ServerState};

use sdk::{api::commands::invite::RedeemInviteBody, models::*};

/*
 * Process:
 *  1. Decrypt code and find invite
 *  2. Check if invite has uses AND that user is not banned from party invite is for
 *  3. Update invite to decrement count
 */

pub async fn redeem_invite(
    state: ServerState,
    auth: Authorization,
    code: SmolStr,
    body: RedeemInviteBody,
) -> Result<(), Error> {
    let maybe_id = crate::backend::util::encrypted_asset::decrypt_snowflake(&state, &code);

    let mut db = state.db.write.get().await?;

    let t = db.transaction().await?;

    // redeem the invite and add user to party_member
    let row = t
        .query_one_cached_typed(
            || {
                use schema::*;
                use thorn::*;

                Query::call(Call::custom("lantern.redeem_invite").args((
                    Var::of(Users::Id),
                    Var::of(Invite::Id),
                    Var::of(Invite::Vanity),
                )))
            },
            &[&auth.user_id, &maybe_id, &code],
        )
        .await;

    let row = match row {
        Ok(row) => row,
        Err(err) => {
            if let Some(db) = err.as_db_error() {
                match *db.code() {
                    SqlState::RAISE_EXCEPTION => match db.message() {
                        "user_banned" => return Err(Error::BadRequest),
                        "invalid_invite" => return Err(Error::NotFound),
                        _ => {}
                    },
                    SqlState::UNIQUE_VIOLATION => match db.constraint() {
                        Some("party_member_pk") => return Err(Error::Conflict),
                        _ => {}
                    },
                    _ => {}
                }
            }

            return Err(err.into());
        }
    };

    let invite_id: Snowflake = row.try_get(0)?;
    let party_id: Snowflake = row.try_get(1)?;

    let mut update_member = Either::Left(futures::future::ok::<_, Error>(()));

    if let Some(nickname) = body.nickname {
        use sdk::api::commands::user::UpdateUserProfileBody;

        update_member = Either::Right(
            crate::backend::api::user::me::profile::patch_profile(
                state.clone(),
                auth,
                UpdateUserProfileBody {
                    nick: Nullable::Some(nickname),
                    ..UpdateUserProfileBody::default()
                },
                Some(party_id),
            )
            .map_ok(|_| ())
            .boxed(),
        );
    }

    let welcome_message = async {
        let msg_id = Snowflake::now();

        t.execute_cached_typed(
            || {
                use schema::*;
                use thorn::*;

                let msg_id = Var::at(Messages::Id, 1);
                let user_id = Var::at(Messages::UserId, 2);
                let msg_kind = Var::at(Messages::Kind, 3);
                let invite_id = Var::at(Invite::Id, 4);

                Query::insert()
                    .into::<Messages>()
                    .cols(&[
                        Messages::Id,
                        Messages::UserId,
                        Messages::Kind,
                        Messages::Content,
                        Messages::RoomId,
                    ])
                    .query(
                        Query::select()
                            .exprs([msg_id, user_id, msg_kind])
                            .expr("".lit())
                            .col(Party::DefaultRoom)
                            .from(Invite::inner_join_table::<Party>().on(Party::Id.equals(Invite::PartyId)))
                            .and_where(Invite::Id.equals(invite_id))
                            .as_value(),
                    )
            },
            &[&msg_id, &auth.user_id, &(MessageKind::Welcome as i16), &invite_id],
        )
        .await?;

        Ok(())
    };

    tokio::try_join!(update_member, welcome_message)?;

    t.commit().await?;

    Ok(())
}
