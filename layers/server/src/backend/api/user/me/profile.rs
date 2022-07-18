use futures::{future::Either, TryFutureExt};
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

    let db = if has_status || has_bio {
        let db = state.db.write.get().await?;

        // try to avoid as many triggers as possible by grouping together queries
        match (has_status, has_bio) {
            (true, true) => {
                db.execute_cached_typed(
                    || insert_or_update_profile(&[Profiles::CustomStatus, Profiles::Biography]),
                    &[&auth.user_id, &party_id, &new_profile.status, &new_profile.bio],
                )
                .await?;
            }
            (true, false) => {
                db.execute_cached_typed(
                    || insert_or_update_profile(&[Profiles::CustomStatus]),
                    &[&auth.user_id, &party_id, &new_profile.status],
                )
                .await?;
            }
            (false, true) => {
                db.execute_cached_typed(
                    || insert_or_update_profile(&[Profiles::Biography]),
                    &[&auth.user_id, &party_id, &new_profile.bio],
                )
                .await?;
            }
            _ => {}
        }

        db
    } else {
        // if we don't need to write now, just acquire a read connection for the next section
        state.db.read.get().await?
    };

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

    // initialize with the avatar/banner file IDs, because if they are Some it will be overwritten, but otherwise inherit the None/Undefined values
    let mut new_avatar_id_future = Either::Left(futures::future::ok(new_profile.avatar));
    let mut new_banner_id_future = Either::Left(futures::future::ok(new_profile.banner));

    if let Nullable::Some(file_id) = new_profile.avatar {
        new_avatar_id_future =
            Either::Right(add_asset(&state, AssetMode::Avatar, auth.user_id, file_id).map_ok(Nullable::Some));
    }

    if let Nullable::Some(file_id) = new_profile.banner {
        new_banner_id_future =
            Either::Right(add_asset(&state, AssetMode::Banner, auth.user_id, file_id).map_ok(Nullable::Some));
    }

    let (new_avatar_id, new_banner_id) = tokio::try_join!(new_avatar_id_future, new_banner_id_future)?;

    let db = state.db.write.get().await?;

    // NOTE: This is kind of in reverse order from the status/bio combinations due to the wildcard matching
    match (new_avatar_id, new_banner_id) {
        (Nullable::Undefined, Nullable::Undefined) => {
            // just set bits...
            db.execute_cached_typed(
                || insert_or_update_profile(&[Profiles::Bits]),
                &[&auth.user_id, &party_id, &new_profile.bits],
            )
            .await?;
        }
        (avatar_id, Nullable::Undefined) => {
            db.execute_cached_typed(
                || insert_or_update_profile(&[Profiles::AvatarId, Profiles::Bits]),
                &[&auth.user_id, &party_id, &avatar_id],
            )
            .await?;
        }
        (Nullable::Undefined, banner_id) => {
            db.execute_cached_typed(
                || insert_or_update_profile(&[Profiles::BannerId, Profiles::Bits]),
                &[&auth.user_id, &party_id, &banner_id],
            )
            .await?;
        }
        (avatar_id, banner_id) => {
            db.execute_cached_typed(
                || insert_or_update_profile(&[Profiles::AvatarId, Profiles::BannerId, Profiles::Bits]),
                &[&auth.user_id, &party_id, &avatar_id, &banner_id],
            )
            .await?;
        }
    }

    Ok(UserProfile {
        bits: new_profile.bits,
        status: new_profile.status,
        bio: new_profile.bio,
        avatar: new_avatar_id.map(|id| encrypt_snowflake(&state, id)),
        banner: new_banner_id.map(|id| encrypt_snowflake(&state, id)),
    })
}

fn insert_or_update_profile(cols: &[schema::Profiles]) -> impl thorn::AnyQuery {
    use schema::*;
    use thorn::conflict::ConflictAction;
    use thorn::*;

    let mut q = Query::insert().into::<Profiles>();

    q = q.cols(&[Profiles::UserId, Profiles::PartyId]).cols(cols);

    q = q.values([Var::at(Profiles::UserId, 1), Var::at(Profiles::PartyId, 2)]);
    q = q.values(cols.iter().enumerate().map(|(v, c)| Var::at(*c, v + 3)));

    // TODO: Make this more ergonomic...
    q = q.on_conflict([Profiles::UserId, Profiles::PartyId], {
        let mut i = cols.iter().enumerate();

        let first = i.next().unwrap();
        let mut action = DoUpdate.set(*first.1, Var::at(*first.1, first.0 + 3));

        for (v, c) in i {
            action = action.set(*c, Var::at(*c, v + 3));
        }

        action
    });

    q
}
