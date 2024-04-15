use sdk::models::*;

use crate::prelude::*;

pub async fn remove_role(
    state: ServerState,
    auth: Authorization,
    party_id: PartyId,
    role_id: RoleId,
) -> Result<(), Error> {
    // cannot remove @everyone role
    if role_id == party_id {
        return Err(Error::BadRequest);
    }

    let db = state.db.read.get().await?;

    #[rustfmt::skip]
    let role_rows = db.query2(schema::sql! {
        SELECT
            Roles.Id AS @RoleId,
            Roles.Position AS @Position,
            Roles.Permissions1 AS @Permissions1,
            Roles.Permissions2 AS @Permissions2,
            EXISTS(
                SELECT FROM RoleMembers
                WHERE RoleMembers.RoleId = Roles.Id
                  AND RoleMembers.UserId = #{auth.user_id_ref() as Users::Id}
            ) AS @HasRole
        FROM Roles
        WHERE Roles.PartyId = #{&party_id as Party::Id}
    }).await?;

    drop(db);

    if role_rows.is_empty() {
        return Err(Error::Unauthorized);
    }

    use schema::roles::{CheckStatus, PartialRole, RoleChecker};

    let mut user_roles = Vec::new();
    let mut roles = Vec::with_capacity(role_rows.len());

    for row in role_rows {
        let id: RoleId = row.role_id()?;

        let role = PartialRole {
            permissions: Permissions::from_i64(row.permissions1()?, row.permissions2()?),
            position: row.position::<i16>()? as u8,
        };

        roles.push((id, role));

        if row.has_role()? {
            user_roles.push(id);
        }
    }

    let target_role = match RoleChecker::new(party_id, roles).check_modify(&user_roles, role_id, None) {
        CheckStatus::Allowed(target_role) => target_role,
        _ => {
            // TODO: improve errors from CheckStatus
            return Err(Error::Unauthorized);
        }
    };

    unimplemented!()
}
