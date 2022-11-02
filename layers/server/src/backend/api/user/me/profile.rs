use arrayvec::ArrayVec;
use futures::{future::Either, FutureExt, TryFutureExt};
use sdk::{api::commands::user::UpdateUserProfileBody, models::*};

use crate::{
    backend::{
        asset::{add_asset, AssetMode},
        util::encrypted_asset::encrypt_snowflake,
    },
    Authorization, Error, ServerState,
};

use schema::Profiles;

pub async fn patch_profile(
    state: ServerState,
    auth: Authorization,
    mut new_profile: UpdateUserProfileBody,
    party_id: Option<Snowflake>,
) -> Result<UserProfile, Error> {
    // if status/bio have a value, insert/update the profile
    let has_status = !new_profile.status.is_undefined();
    let has_bio = !new_profile.bio.is_undefined();
    let has_nick = !new_profile.nick.is_undefined();

    let db = if has_status || has_bio { state.db.write.get().await? } else { state.db.read.get().await? };

    if party_id.is_some() {
        // TODO: Add permissions?
        let is_member = db
            .query_opt_cached_typed(
                || {
                    use schema::*;
                    use thorn::*;

                    Query::select()
                        .expr(1.lit())
                        .from_table::<PartyMember>()
                        .and_where(PartyMember::UserId.equals(Var::of(Users::Id)))
                        .and_where(PartyMember::PartyId.equals(Var::of(Party::Id)))
                },
                &[&auth.user_id, &party_id],
            )
            .await?
            .is_some();

        if !is_member {
            return Err(Error::Unauthorized);
        }
    }

    // try to avoid as many triggers as possible by grouping together queries
    #[rustfmt::skip]
    let prepared_stmt = match (has_status, has_bio, has_nick) {
        (true,  true,  false) => db.prepare_cached_typed(|| insert_or_update_profile(&[Profiles::Bits, Profiles::CustomStatus, Profiles::Biography])).boxed(),
        (true,  false, false) => db.prepare_cached_typed(|| insert_or_update_profile(&[Profiles::Bits, Profiles::CustomStatus])).boxed(),
        (false, true,  false) => db.prepare_cached_typed(|| insert_or_update_profile(&[Profiles::Bits, Profiles::Biography])).boxed(),
        (true,  true,  true)  => db.prepare_cached_typed(|| insert_or_update_profile(&[Profiles::Bits, Profiles::CustomStatus, Profiles::Biography, Profiles::Nickname])).boxed(),
        (true,  false, true)  => db.prepare_cached_typed(|| insert_or_update_profile(&[Profiles::Bits, Profiles::CustomStatus, Profiles::Nickname])).boxed(),
        (false, true,  true)  => db.prepare_cached_typed(|| insert_or_update_profile(&[Profiles::Bits, Profiles::Biography, Profiles::Nickname])).boxed(),
        (false, false, true)  => db.prepare_cached_typed(|| insert_or_update_profile(&[Profiles::Bits, Profiles::Nickname])).boxed(),
        (false, false, false) => db.prepare_cached_typed(|| insert_or_update_profile(&[Profiles::Bits])).boxed(),
    };

    let mut params = ArrayVec::<&(dyn db::pg::types::ToSql + Sync), 6>::new();

    params.push(&auth.user_id);
    params.push(&party_id);
    params.push(&new_profile.bits);

    if has_status {
        params.push(&new_profile.status);
    }

    if has_bio {
        params.push(&new_profile.bio);
    }

    if has_nick {
        params.push(&new_profile.nick);
    }

    db.execute(&prepared_stmt.await?, &params).await?;

    drop(params);

    // using the existing database connection, go ahead and fetch the old file ids

    let row = db
        .query_opt_cached_typed(
            || {
                use schema::*;
                use thorn::*;

                Query::select()
                    .and_where(
                        AggOriginalProfileFiles::UserId.equals(Var::of(AggOriginalProfileFiles::UserId)),
                    )
                    .and_where(
                        AggOriginalProfileFiles::PartyId.equals(Var::of(AggOriginalProfileFiles::PartyId)),
                    )
                    .from_table::<AggOriginalProfileFiles>()
                    .cols(&[
                        AggOriginalProfileFiles::AvatarFileId,
                        AggOriginalProfileFiles::BannerFileId,
                    ])
            },
            &[&auth.user_id, &party_id],
        )
        .await?;

    drop(db);

    let (old_avatar_id, old_banner_id) = match row {
        None => (None, None),
        Some(row) => (
            row.try_get::<_, Option<Snowflake>>(0)?,
            row.try_get::<_, Option<Snowflake>>(1)?,
        ),
    };

    // avoid reprocessing the same files if they were somehow resent

    if let Some(old_avatar_id) = old_avatar_id {
        if Nullable::Some(old_avatar_id) == new_profile.avatar {
            new_profile.avatar = Nullable::Undefined; // unchanged
        }
    }

    if let Some(old_banner_id) = old_banner_id {
        if Nullable::Some(old_banner_id) == new_profile.banner {
            new_profile.banner = Nullable::Undefined; // unchanged
        }
    }

    let (new_avatar_id, new_banner_id) = if !new_profile.avatar.is_undefined()
        || !new_profile.banner.is_undefined()
    {
        // initialize with the avatar/banner file IDs, because if they are Some it will be overwritten, but otherwise inherit the None/Undefined values
        let mut new_avatar_id_future = Either::Left(futures::future::ok(new_profile.avatar));
        let mut new_banner_id_future = Either::Left(futures::future::ok(new_profile.banner));

        if let Nullable::Some(file_id) = new_profile.avatar {
            new_avatar_id_future = Either::Right(
                add_asset(&state, AssetMode::Avatar, auth.user_id, file_id).map_ok(Nullable::Some),
            );
        }

        if let Nullable::Some(file_id) = new_profile.banner {
            new_banner_id_future = Either::Right(
                add_asset(&state, AssetMode::Banner, auth.user_id, file_id).map_ok(Nullable::Some),
            );
        }

        let (new_avatar_id, new_banner_id) = tokio::try_join!(new_avatar_id_future, new_banner_id_future)?;

        let db = state.db.write.get().await?;

        match (new_avatar_id, new_banner_id) {
            (Nullable::Undefined, Nullable::Undefined) => {}
            (avatar_id, Nullable::Undefined) => {
                db.execute_cached_typed(
                    || insert_or_update_profile(&[Profiles::AvatarId, Profiles::Bits]),
                    &[&auth.user_id, &party_id, &avatar_id, &new_profile.bits],
                )
                .await?;
            }
            (Nullable::Undefined, banner_id) => {
                db.execute_cached_typed(
                    || insert_or_update_profile(&[Profiles::BannerId, Profiles::Bits]),
                    &[&auth.user_id, &party_id, &banner_id, &new_profile.bits],
                )
                .await?;
            }
            (avatar_id, banner_id) => {
                db.execute_cached_typed(
                    || insert_or_update_profile(&[Profiles::AvatarId, Profiles::BannerId, Profiles::Bits]),
                    &[
                        &auth.user_id,
                        &party_id,
                        &avatar_id,
                        &banner_id,
                        &new_profile.bits,
                    ],
                )
                .await?;
            }
        }

        (new_avatar_id, new_banner_id)
    } else {
        (new_profile.avatar, new_profile.banner)
    };

    Ok(UserProfile {
        bits: new_profile.bits,
        extra: Default::default(),
        nick: new_profile.nick,
        status: new_profile.status,
        bio: new_profile.bio,
        avatar: new_avatar_id.map(|id| encrypt_snowflake(&state, id)),
        banner: new_banner_id.map(|id| encrypt_snowflake(&state, id)),
    })
}

fn insert_or_update_profile(cols: &[schema::Profiles]) -> impl thorn::AnyQuery {
    use schema::*;
    use thorn::conflict::ConflictAction;
    use thorn::table::ColumnExt;
    use thorn::*;

    let user_id_var = Var::at(Profiles::UserId, 1);
    let party_id_var = Var::at(Profiles::PartyId, 2);

    let mut q = Query::insert().into::<Profiles>();

    q = q.cols(&[Profiles::UserId, Profiles::PartyId]).cols(cols);

    q = q.values([user_id_var, party_id_var]);
    q = q.values(cols.iter().enumerate().map(|(v, c)| Var::at(*c, v + 3)));

    // TODO: Make this more ergonomic...
    q = q.on_expr_conflict(
        [
            Box::new(Profiles::UserId.as_name_only()) as Box<dyn Expr>,
            Box::new(Builtin::coalesce((Profiles::PartyId.as_name_only(), 1i64.lit()))) as Box<dyn Expr>,
        ],
        {
            let mut i = cols.iter().enumerate();

            let first = i.next().unwrap();
            let mut action = DoUpdate.set(*first.1, Var::at(*first.1, first.0 + 3));

            for (v, c) in i {
                action = action.set(*c, Var::at(*c, v + 3));
            }

            action
        },
    );

    q
}
