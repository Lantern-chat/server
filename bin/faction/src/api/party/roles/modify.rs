use futures::TryFutureExt;
use sdk::{api::commands::party::PatchRoleForm, models::*};

use crate::{
    asset::{maybe_add_asset, AssetMode},
    prelude::*,
    util::encrypted_asset::encrypt_snowflake_opt,
};

pub async fn modify_role(
    state: ServerState,
    auth: Authorization,
    party_id: Snowflake,
    role_id: Snowflake,
    mut form: PatchRoleForm,
) -> Result<Role, Error> {
    // TODO: Maybe change this?
    if form == PatchRoleForm::default() {
        return Err(Error::BadRequest);
    }

    if matches!(form.name, Some(ref name) if !state.config().party.role_name_len.contains(&name.len())) {
        return Err(Error::InvalidName);
    }

    let has_assets = form.avatar.is_some();

    let db = state.db.read.get().await?;

    #[rustfmt::skip]
    let role_rows = db.query2(schema::sql! {
        SELECT
            Roles.Id AS @RoleId,
            Roles.Position AS @Position,
            Roles.Permissions1 AS @Permissions1,
            Roles.Permissions2 AS @Permissions2,
            if has_assets { UserAssets.FileId } else { NULL } AS @AvatarFileId,
            EXISTS(
                SELECT FROM RoleMembers
                WHERE RoleMembers.RoleId = Roles.Id
                  AND RoleMembers.UserId = #{auth.user_id_ref() as Users::Id}
            ) AS @HasRole

        FROM Roles
        if has_assets {
            LEFT JOIN UserAssets ON UserAssets.Id = Roles.AvatarId
        }
        WHERE Roles.PartyId = #{&party_id as Party::Id}
    }).await?;

    drop(db);

    if role_rows.is_empty() {
        return Err(Error::Unauthorized);
    }

    use schema::roles::{CheckStatus, PartialRole, RoleChecker};

    let mut user_roles = Vec::new();
    let mut roles = Vec::with_capacity(role_rows.len());
    let mut existing_avatar_file_id: Option<Snowflake> = None;

    for row in role_rows {
        let id: Snowflake = row.role_id()?;

        let role = PartialRole {
            permissions: Permissions::from_i64(row.permissions1()?, row.permissions2()?),
            position: row.position::<i16>()? as u8,
        };

        if id == role_id {
            existing_avatar_file_id = row.avatar_file_id()?;
        }

        roles.push((id, role));

        if row.has_role()? {
            user_roles.push(id);
        }
    }

    let checker = RoleChecker::new(party_id, roles);

    let target_role = match checker.check_modify(&user_roles, role_id, Some(&form)) {
        CheckStatus::Allowed(target_role) => target_role,
        _ => {
            // TODO: improve errors from CheckStatus
            return Err(Error::Unauthorized);
        }
    };

    // no change, don't reprocess avatar
    if matches!((form.avatar, existing_avatar_file_id), (Nullable::Some(a), Some(b)) if a == b) {
        form.avatar = Nullable::Undefined;
    }

    let new_position = match form.position {
        Some(position) => {
            if position == target_role.position {
                form.position = None;
            }

            position as i16
        }
        _ => target_role.position as i16,
    };

    if form == PatchRoleForm::default() {
        // TODO: return success instead since it passes but it's a no-op?
        return Err(Error::BadRequest);
    }

    let color = form.color.map(|c| c as i32);
    let avatar_id = maybe_add_asset(&state, AssetMode::Avatar, auth.user_id(), form.avatar).await?;
    let [perms1, perms2] = match form.permissions {
        Some(perms) => perms.to_i64(),
        None => [0, 0], // unused
    };

    let mut db = state.db.write.get().await?;
    let t = db.transaction().await?;

    #[rustfmt::skip]
    let updating_role = t.query_one2(schema::sql! {
        UPDATE Roles SET
            if form.name.is_some()        { Roles./Name     = #{&form.name as Roles::Name}, }
            if !avatar_id.is_undefined()  { Roles./AvatarId = #{&avatar_id as Roles::AvatarId}, }
            if color.is_some()            { Roles./Color    = #{&color as Roles::Color}, }
            if form.flags.is_some()       { Roles./Flags    = #{&form.flags as Roles::Flags}, }
            if form.permissions.is_some() {
                Roles./Permissions1 = #{&perms1 as Roles::Permissions1},
                Roles./Permissions2 = #{&perms2 as Roles::Permissions2}
            }

            Roles./Position = #{&new_position as Roles::Position}
        WHERE Roles.Id = #{&role_id as Roles::Id}
        RETURNING
            Roles.AvatarId      AS @AvatarId,
            Roles.Name          AS @Name,
            Roles.Permissions1  AS @Permissions1,
            Roles.Permissions2  AS @Permissions2,
            Roles.Color         AS @Color,
            Roles.Position      AS @Position,
            Roles.Flags         AS @Flags
    }).map_err(Error::from);

    let updating_role_positions = async {
        // no movement, do nothing
        if (new_position as u8) == target_role.position {
            return Ok(());
        }

        Err(Error::Unimplemented)
    };

    let (row, _) = tokio::try_join!(updating_role, updating_role_positions)?;

    t.commit().await?;

    Ok(Role {
        id: role_id,
        party_id,
        avatar: encrypt_snowflake_opt(&state, row.avatar_id()?),
        name: row.name()?,
        desc: None, // TODO
        permissions: Permissions::from_i64(row.permissions1()?, row.permissions2()?),
        color: row.color::<Option<i32>>()?.map(|c| c as u32),
        position: row.position()?,
        flags: row.flags()?,
    })
}
