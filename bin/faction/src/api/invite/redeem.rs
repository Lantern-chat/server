use db::pg::error::SqlState;
use futures::{future::Either, FutureExt, TryFutureExt};

use schema::{Snowflake, SnowflakeExt};
use smol_str::SmolStr;

use crate::prelude::*;
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
    let maybe_id = crate::util::encrypted_asset::decrypt_snowflake(&state, &code);

    let mut db = state.db.write.get().await?;

    let t = db.transaction().await?;

    // redeem the invite and add user to party_member
    let row = t.query_one2(schema::sql! {
        CALL .redeem_invite(
            #{auth.user_id_ref() as Users::Id},
            #{&maybe_id as Invite::Id},
            #{&code as Invite::Vanity}
        )
    });

    let row = match row.await {
        Ok(row) => row,
        Err(err) => {
            if let Some(db) = err.as_db_error() {
                match *db.code() {
                    SqlState::RAISE_EXCEPTION => match db.message() {
                        "user_banned" => return Err(Error::Unauthorized),
                        "invalid_invite" => return Err(Error::NotFound),
                        _ => {}
                    },
                    SqlState::UNIQUE_VIOLATION => match db.constraint() {
                        Some("party_members_pk") => return Err(Error::Conflict),
                        _ => todo!("Other constraints"),
                    },
                    _ => {}
                }
            }

            return Err(err.into());
        }
    };

    let invite_id: Snowflake = row.try_get(0)?;
    let party_id: Snowflake = row.try_get(1)?;

    let update_member = async {
        if let Some(nickname) = body.nickname {
            crate::api::user::me::profile::patch_profile(
                state.clone(),
                auth,
                Some(party_id),
                todo!(), //sdk::api::commands::user::UpdateUserProfileBody {
                         //    nick: Nullable::Some(nickname),
                         //    ..Default::default()
                         //},
            )
            // avoid inlining this future
            .boxed()
            .await?;
        }

        Ok::<_, Error>(())
    };

    let welcome_message = async {
        let msg_id = state.sf.gen();

        t.execute2(schema::sql! {
            INSERT INTO Messages (Id, UserId, RoomId, Kind) (
                SELECT
                    #{&msg_id as Messages::Id},
                    #{auth.user_id_ref() as Messages::UserId},
                    Party.DefaultRoom,
                    {MessageKind::Welcome as i16}
                FROM Invite INNER JOIN LiveParties AS Party ON Party.Id = Invite.PartyId
                WHERE Invite.Id = #{&invite_id as Invite::Id}
            )
        })
        .await?;

        Ok(())
    };

    tokio::try_join!(update_member, welcome_message)?;

    t.commit().await?;

    Ok(())
}
