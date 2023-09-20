use schema::SnowflakeExt;

use sdk::{api::commands::party::CreateRoleForm, models::*};

use crate::{Authorization, Error, ServerState};

pub async fn create_role(
    state: ServerState,
    auth: Authorization,
    party_id: Snowflake,
    form: CreateRoleForm,
) -> Result<Role, Error> {
    {
        let config = state.config();

        if !config.party.role_name_len.contains(&form.name.len()) {
            return Err(Error::InvalidName);
        }
    }

    let role_id = Snowflake::now();

    let mut db = state.db.write.get().await?;
    let t = db.transaction().await?;

    #[rustfmt::skip]
    let row = t.query_one2(schema::sql! {
        tables! {
            struct TempRole {
                Id: Roles::Id,
                Position: Roles::Position,
                Perms1: Roles::Permissions1,
                Perms2: Roles::Permissions2,
            }
            struct Inserted { Position: Roles::Position }
        };

        WITH TempRole AS (
            SELECT
                #{&role_id as TempRole::Id} AS TempRole.Id,
                Roles.Permissions1 AS TempRole.Perms1,
                Roles.Permissions2 AS TempRole.Perms2,
                (
                    SELECT MAX(Roles.Position) FROM Roles
                    WHERE Roles.PartyId = PartyMembers.PartyId
                ) AS TempRole.Position
             FROM PartyMembers INNER JOIN Roles ON Roles.Id = PartyMembers.PartyId
            WHERE PartyMembers.PartyId = #{&party_id as Party::Id}
              AND PartyMembers.UserId = #{auth.user_id_ref() as Users::Id}

            let perms = Permissions::MANAGE_ROLES.to_i64();
            assert_eq!(perms[1], 0);
            AND PartyMembers.Permissions1 & {perms[0]} = {perms[0]}
        ), Inserted AS (
            INSERT INTO Roles (Id, Position, PartyId, Name, Permissions1, Permissions2) (
                SELECT TempRole.Id, TempRole.Position + 1,
                       #{&party_id  as Roles::PartyId},
                       #{&form.name as Roles::Name},
                       TempRole.Perms1, TempRole.Perms2
                // TODO: Handle this limit better
                WHERE TempRole.Position < 255
            )
            RETURNING Roles.Position AS Inserted.Position
        )
        SELECT
            TempRole.Id AS @RoleId,
            Inserted.Position AS @Position
        FROM TempRole LEFT JOIN Inserted ON TRUE
    }).await?;

    if row.role_id::<Option<Snowflake>>()?.is_none() {
        t.rollback().await?;
        return Err(Error::Unauthorized);
    };

    let Some(position) = row.position()? else {
        t.rollback().await?;

        // TODO: Error for too many roles
        return Err(Error::BadRequest);
    };

    t.commit().await?;

    Ok(Role {
        id: role_id,
        party_id,
        avatar: None,
        name: form.name,
        permissions: Permissions::empty(),
        color: None,
        position,
        flags: RoleFlags::empty(),
    })
}
