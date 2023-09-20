//! [`RoleChecker`] tool.
//!
//! Rules:
//! - Cannot edit roles at or above their own highest role
//! -    This includes moving a role to become higher than their own.
//! - Can only assign permissions they have
//! - Cannot remove permissions that would remove it from themselves
//! - Cannot ban/kick/rename users with roles at or above their own
//! - Admins are exempt from the assign/remove safety restrictions. Can freely give or remove for any role below them.

use sdk::api::commands::party::PatchRoleForm;
use sdk::models::{Permissions, Snowflake};

use fxhash::FxBuildHasher;
use indexmap::IndexMap;

#[derive(Debug, Clone, Copy)]
pub struct PartialRole {
    pub permissions: Permissions,
    pub position: u8,
}

pub struct RoleChecker {
    roles: IndexMap<Snowflake, PartialRole, FxBuildHasher>,
    party_id: Snowflake,
}

#[derive(Debug, Clone, Copy)]
pub enum CheckStatus {
    Allowed,

    /// The target role wasn't found
    NotFound,

    /// You have no permissions to manage roles
    NoPerms,

    /// The role you are targetting is above your rank
    AboveRank,

    /// Removing a permission would remove it from yourself
    InvalidRemoval,

    /// Cannot grant permissions you do not have
    InvalidAddition,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UserAction {
    Kick,
    Ban,
    Rename,
}

impl RoleChecker {
    pub fn new(party_id: Snowflake, roles: impl IntoIterator<Item = (Snowflake, PartialRole)>) -> Self {
        let mut roles = IndexMap::from_iter(roles);

        roles.sort_unstable_by(|a_id, a, b_id, b| (a.position, *a_id).cmp(&(b.position, *b_id)));

        RoleChecker { roles, party_id }
    }

    fn everyone(&self) -> (usize, &Snowflake, &PartialRole) {
        self.roles.get_full(&self.party_id).expect("Unable to find @everyone role")
    }

    /// Checks if you have the ability to kick/ban/rename
    pub fn check_user(
        &self,
        user_roles: &[Snowflake],
        other_roles: &[Snowflake],
        action: UserAction,
    ) -> CheckStatus {
        if user_roles.is_empty() {
            return CheckStatus::NoPerms;
        }

        let (everyone, ..) = self.everyone();

        let mut highest_own = self.roles.get_index_of(&user_roles[0]).unwrap_or(everyone);
        let mut highest_other = match other_roles.is_empty() {
            false => everyone,
            true => self.roles.get_index_of(&other_roles[0]).unwrap_or(everyone),
        };

        let mut permissions = Permissions::empty();

        for role_id in user_roles {
            let Some((idx, _, role)) = self.roles.get_full(role_id) else {
                continue;
            };

            permissions |= role.permissions;
            highest_own = highest_own.min(idx); // "highest" is sorted first, so min index
        }

        if permissions.contains(Permissions::ADMINISTRATOR) {
            permissions = Permissions::all();
        }

        let required_perm = match action {
            UserAction::Kick => Permissions::KICK_MEMBERS,
            UserAction::Ban => Permissions::BAN_MEMBERS,
            UserAction::Rename => Permissions::MANAGE_NICKNAMES,
        };

        if !permissions.contains(required_perm) {
            return CheckStatus::NoPerms;
        }

        for role_id in other_roles {
            let Some(idx) = self.roles.get_index_of(role_id) else {
                continue;
            };

            highest_other = highest_other.min(idx);
        }

        if highest_other <= highest_own {
            return CheckStatus::AboveRank;
        }

        CheckStatus::Allowed
    }

    pub fn check_modify(&self, user_roles: &[Snowflake], role_id: Snowflake, form: &PatchRoleForm) -> CheckStatus {
        let Some(target_role_idx) = self.roles.get_index_of(&role_id) else {
            return CheckStatus::NotFound;
        };

        let (everyone, ..) = self.everyone();

        let mut permissions = Permissions::empty();
        let mut highest = everyone;

        for role_id in user_roles {
            let Some((idx, _, role)) = self.roles.get_full(role_id) else {
                continue;
            };

            permissions |= role.permissions;
            highest = highest.min(idx); // "highest" is sorted first, so min index
        }

        if permissions.contains(Permissions::ADMINISTRATOR) {
            permissions = Permissions::all();
        }

        if !permissions.contains(Permissions::MANAGE_ROLES) {
            return CheckStatus::NoPerms;
        };

        if target_role_idx <= highest {
            return CheckStatus::AboveRank;
        }

        if let Some(new_permissions) = form.permissions {
            // cannot assign permissions you don't have
            if !permissions.contains(new_permissions) {
                return CheckStatus::InvalidAddition;
            }

            // check that the user is not removing their own permissions
            if user_roles.contains(&role_id) {
                let removed_permissions = permissions - new_permissions;

                if !removed_permissions.is_empty() {
                    let mut count = 0;

                    for role_id in user_roles {
                        let Some(role) = self.roles.get(role_id) else { continue };

                        count += role.permissions.contains(removed_permissions) as usize;

                        if count == 2 {
                            break;
                        }
                    }

                    if count == 1 {
                        return CheckStatus::InvalidRemoval;
                    }
                }
            }
        }

        if let Some(position) = form.position {
            // cannot move a role to be higher than your highest role (inverted priority)
            let (_, highest) = self.roles.get_index(highest).unwrap();
            if position <= highest.position {
                return CheckStatus::AboveRank;
            }
        }

        CheckStatus::Allowed
    }
}
