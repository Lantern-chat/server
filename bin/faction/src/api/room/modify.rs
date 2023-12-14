use hashbrown::HashSet;
use schema::Snowflake;

use crate::api::party::rooms::create::RawOverwrites;
use crate::asset::{maybe_add_asset, AssetMode};
use crate::prelude::*;
use sdk::api::commands::room::PatchRoomForm;
use sdk::models::*;

pub async fn modify_room(
    state: ServerState,
    auth: Authorization,
    room_id: Snowflake,
    mut form: PatchRoomForm,
) -> Result<FullRoom, Error> {
    // TODO: Maybe change this?
    if form == PatchRoomForm::default() {
        return Err(Error::BadRequest);
    }

    let name;
    {
        let config = state.config();
        if matches!(form.topic, Nullable::Some(ref topic) if !config.shared.room_topic_length.contains(&topic.len()))
        {
            return Err(Error::InvalidTopic);
        }

        name = form.name.as_ref().map(|name| schema::names::slug_name(name));

        if matches!(name, Some(ref name) if !schema::validation::validate_name(name, config.shared.room_name_length.clone()))
        {
            return Err(Error::InvalidName);
        }
    }

    let has_assets = form.avatar.is_some();

    let mut old_avatar_id: Nullable<Snowflake> = Nullable::Null;

    let mut needs_perms = true;
    if let Some(perms) = state.perm_cache.get(auth.user_id(), room_id).await {
        if !perms.contains(Permissions::MANAGE_ROOMS) {
            return Err(Error::Unauthorized);
        }

        needs_perms = false;
    }

    if needs_perms || has_assets {
        #[rustfmt::skip]
        let Some(row) = state.db.read.get().await?.query_opt2(schema::sql! {
            SELECT
                Rooms.Permissions1 AS @Permissions1,
                Rooms.Permissions2 AS @Permissions2,
                if has_assets { UserAssets.FileId } else { NULL } AS @AvatarFileId
            FROM AggRoomPerms AS Rooms if has_assets {
                LEFT JOIN UserAssets ON UserAssets.Id = Rooms.AvatarId
            }
            WHERE Rooms.UserId = #{auth.user_id_ref() as Users::Id}
              AND Rooms.Id = #{&room_id as Rooms::Id}
        }).await? else {
            return Err(Error::Unauthorized);
        };

        if !Permissions::from_i64(row.permissions1()?, row.permissions2()?).contains(Permissions::MANAGE_ROOMS) {
            return Err(Error::Unauthorized);
        }

        old_avatar_id = row.avatar_file_id()?;
    }

    if has_assets && old_avatar_id == form.avatar {
        form.avatar = Nullable::Undefined;
    }

    let avatar_id = maybe_add_asset(&state, AssetMode::Avatar, auth.user_id(), form.avatar).await?;
    let position = form.position.map(|p| p as i16);

    // unique + avoiding conflicts
    if !form.remove_overwrites.is_empty() {
        let mut to_remove: HashSet<Snowflake> = HashSet::from_iter(form.remove_overwrites);

        for ow in &form.overwrites {
            to_remove.remove(&ow.id);
        }

        form.remove_overwrites = to_remove.into_iter().collect();
    }

    let raw = RawOverwrites::new(form.overwrites);

    let mut db = state.db.write.get().await?;
    let t = db.transaction().await?;

    let remove_overwrites = async {
        if form.remove_overwrites.is_empty() {
            return Ok(0);
        }

        t.execute2(schema::sql! {
            DELETE FROM Overwrites
            WHERE Overwrites.UserId = ANY(#{&form.remove_overwrites as SNOWFLAKE_ARRAY})
               OR Overwrites.RoleId = ANY(#{&form.remove_overwrites as SNOWFLAKE_ARRAY})
        })
        .await
    };

    let insert_overwrites = async {
        if raw.id.is_empty() {
            return Ok(0);
        }

        t.execute2(schema::sql! {
            tables! {
                struct Ow {
                    Id: SNOWFLAKE,
                    Allow1: Type::INT8,
                    Allow2: Type::INT8,
                    Deny1:  Type::INT8,
                    Deny2:  Type::INT8,
                }
            };

            WITH Ow AS (
                SELECT
                    UNNEST(#{&raw.id as SNOWFLAKE_ARRAY}) AS Ow.Id,
                    NULLIF(UNNEST(#{&raw.a1 as Type::INT8_ARRAY}), 0) AS Ow.Allow1,
                    NULLIF(UNNEST(#{&raw.a2 as Type::INT8_ARRAY}), 0) AS Ow.Allow2,
                    NULLIF(UNNEST(#{&raw.d1 as Type::INT8_ARRAY}), 0) AS Ow.Deny1,
                    NULLIF(UNNEST(#{&raw.d2 as Type::INT8_ARRAY}), 0) AS Ow.Deny2
            )
            INSERT INTO Overwrites (UserId, RoleId, RoomId, Allow1, Allow2, Deny1, Deny2) (
                // restrict these to user ids found within the same party as this room
                SELECT Ow.Id, NULL, Rooms.Id, Ow.Allow1, Ow.Allow2, Ow.Deny1, Ow.Deny2
                FROM Ow INNER JOIN PartyMembers ON PartyMembers.UserId = Ow.Id
                        INNER JOIN Rooms ON Rooms.PartyId = PartyMembers.PartyId
                WHERE Rooms.Id = #{&room_id as Rooms::Id}

                UNION ALL

                // restrict these to role ids found within the same party as this room
                SELECT NULL, Ow.Id, Rooms.Id, Ow.Allow1, Ow.Allow2, Ow.Deny1, Ow.Deny2
                FROM Ow INNER JOIN Roles ON Roles.Id = Ow.Id
                        INNER JOIN Rooms ON Rooms.PartyId = Roles.PartyId
                WHERE Rooms.Id = #{&room_id as Rooms::Id}
            )
            ON CONFLICT DO UPDATE Overwrites SET (Allow1, Allow2, Deny1, Deny2) = (
                Ow.Allow1, Ow.Allow2, Ow.Deny1, Ow.Deny2
            )
        })
        .await
    };

    let (removed, inserted) = tokio::try_join!(remove_overwrites, insert_overwrites)?;

    if removed != form.remove_overwrites.len() as u64 || inserted != raw.id.len() as u64 {
        t.rollback().await?;

        return Err(Error::BadRequest);
    }

    #[rustfmt::skip]
    let res = t.execute2(schema::sql! {
        UPDATE Rooms SET
            if name.is_some()             { Rooms./Name     = #{&name       as Rooms::Name}, }
            if position.is_some()         { Rooms./Position = #{&position   as Rooms::Position}, }
            if !form.topic.is_undefined() { Rooms./Topic    = #{&form.topic as Rooms::Topic}, }
            if !avatar_id.is_undefined()  { Rooms./AvatarId = #{&avatar_id  as Rooms::AvatarId}, }
            Rooms./Flags = match form.nsfw {
                Some(true)  => { Rooms./Flags |  {RoomFlags::NSFW.bits()} },
                Some(false) => { Rooms./Flags & ~{RoomFlags::NSFW.bits()} },
                None        => { Rooms./Flags }
            }
        WHERE Rooms.Id = #{&room_id as Rooms::Id}
    }).await?;

    if res != 1 {
        t.rollback().await?;

        return Err(Error::InternalErrorStatic("Unable to update room"));
    }

    t.commit().await?;

    crate::api::room::get::get_room(state, auth, room_id).await
}
