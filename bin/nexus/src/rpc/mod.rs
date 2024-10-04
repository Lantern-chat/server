#![allow(clippy::redundant_closure)]

//pub mod admin;
pub mod auth;
pub mod perm;

#[derive(Debug, Clone, Copy)]
pub enum SearchMode<'a> {
    Single(schema::Snowflake),
    Many(&'a [schema::Snowflake]),
}

pub mod user {
    pub mod user_get_user;
    pub mod user_login;
    pub mod user_register;

    pub mod me {
        pub mod user_account;
        pub mod user_change_password;

        pub mod user_get_self;
        pub mod user_logout;
        pub mod user_mfa;
        pub mod user_prefs;
        pub mod user_profile;
        pub mod user_sessions;

        pub mod user_relationships {
            pub mod get_relationships;
            pub mod modify_relationship;
        }
    }
}

pub mod party {
    pub mod party_create;
    pub mod party_emotes;
    pub mod party_get;
    pub mod party_member_profile;
    pub mod party_members;
    pub mod party_modify;
    pub mod party_remove;
    pub mod party_stats;

    pub mod rooms {
        pub mod create_room;
        pub mod get_rooms;
    }

    pub mod roles {
        pub mod create_role;
        pub mod get_roles;
        pub mod modify_role;
        pub mod remove_role;
    }
}

pub mod room {
    pub mod get_room;
    pub mod modify_room;
    pub mod remove_room;
    pub mod start_typing;

    pub mod messages {
        pub mod create_message;
        pub mod delete_message;
        pub mod edit_message;
        pub mod get_messages;

        pub mod reactions {
            pub mod add_reaction;
            pub mod remove_reaction;
        }
    }

    pub mod threads {
        pub mod edit;
        pub mod get;
    }
}

pub mod invite {
    pub mod invite_create;
    pub mod invite_get;
    pub mod invite_redeem;
    pub mod invite_revoke;
}

/*
pub mod file {
    pub mod delete;
    pub mod head;
    pub mod options;
    pub mod patch;
    pub mod post;
}

pub mod metrics;

pub mod oembed {
    pub mod get;
}
*/

use crate::prelude::*;

use rpc::request::ArchivedRpcRequest;
use tokio::io::AsyncWrite;

pub async fn dispatch<W>(state: ServerState, out: W, cmd: &ArchivedRpcRequest) -> Result<(), Error>
where
    W: AsyncWrite + Unpin + Send,
{
    // avoid inlining every async state machine by boxing them inside a lazy future/async block
    macro_rules! c {
        ($([$size:literal])? $first:ident$(::$frag:ident)+($($args:expr),*)) => {
            Box::pin(async move { ::rpc::stream::encode_item::<_, Error, _>(
                out, crate::rpc::$first$(::$frag)+($($args),*).await).await.map_err(Error::from) })
        };
    }
    macro_rules! s {
        ($([$size:literal])? $first:ident$(::$frag:ident)+($($args:expr),*)) => {
            Box::pin(async move { ::rpc::stream::encode_stream::<_, Error, _>(
                out, crate::rpc::$first$(::$frag)+($($args),*).await).await.map_err(Error::from) })
        };
    }

    use core::{future::Future, pin::Pin};

    #[allow(clippy::type_complexity)]
    // using a closure here allows for early returns of the future for auth and others
    let gen_dispatch = move || -> Result<Pin<Box<dyn Future<Output = Result<(), Error>> + Send>>, Error> {
        let (addr, auth, proc) = match cmd {
            ArchivedRpcRequest::Procedure { addr, auth, proc } => (addr, auth, proc),
            ArchivedRpcRequest::Authorize { token } => return Ok(c!(auth::do_auth(state, token))),
            _ => unimplemented!(),
        };

        // prepare fields
        let addr = addr.as_ipaddr();
        let auth = || match auth.as_ref() {
            Some(auth) => Ok(auth.get().deserialize_simple().expect("Unable to deserialize auth")),
            None => Err(Error::Unauthorized),
        };

        use rpc::procedure::ArchivedProcedure as Proc;

        // assigning to a variable is apparently requires to get the type coalescing to work
        #[allow(unused_variables)]
        #[rustfmt::skip]
        let running: Pin<Box<dyn Future<Output = Result<(), Error>> + Send>> = match proc {
            Proc::GetServerConfig(cmd) => todo!("GetServerConfig"),
            Proc::UserRegister(cmd) => c!(user::user_register::register_user(state, addr, cmd)),
            Proc::UserLogin(cmd) => c!(user::user_login::login(state, addr, cmd)),
            Proc::UserLogout(_) => c!(user::me::user_logout::logout_user(state, auth()?)),
            Proc::Enable2FA(cmd) => c!(user::me::user_mfa::enable_2fa(state, auth()?, cmd)),
            Proc::Confirm2FA(cmd) => c!(user::me::user_mfa::confirm_2fa(state, auth()?, cmd)),
            Proc::Remove2FA(cmd) => c!(user::me::user_mfa::remove_2fa(state, auth()?, cmd)),
            Proc::ChangePassword(cmd) => todo!("ChangePassword"),
            Proc::GetSessions(cmd) => s!(user::me::user_sessions::list_sessions(state, auth()?)),
            Proc::ClearSessions(cmd) => c!(user::me::user_sessions::clear_other_sessions(state, auth()?)),
            Proc::GetRelationships(cmd) => todo!("GetRelationships"),
            Proc::PatchRelationship(cmd) => todo!("PatchRelationship"),
            Proc::UpdateUserProfile(cmd) => c!(user::me::user_profile::patch_user_profile(state, auth()?, cmd)),
            Proc::GetUser(cmd) => c!(user::user_get_user::get_full_user(state, auth()?, cmd)),
            Proc::UpdateUserPrefs(cmd) => c!(user::me::user_prefs::update_prefs(state, auth()?, cmd)),
            Proc::CreateFile(cmd) => todo!("CreateFile"),
            Proc::GetFilesystemStatus(cmd) => todo!("GetFilesystemStatus"),
            Proc::GetFileStatus(cmd) => todo!("GetFileStatus"),
            Proc::GetInvite(cmd) => todo!("GetInvite"),
            Proc::RevokeInvite(cmd) => todo!("RevokeInvite"),
            Proc::RedeemInvite(cmd) => todo!("RedeemInvite"),
            Proc::CreateParty(cmd) => c!(party::party_create::create_party(state, auth()?, cmd)),
            Proc::GetParty(cmd) => c!(party::party_get::get_party(state, auth()?, cmd)),
            Proc::PatchParty(cmd) => c!(party::party_modify::modify_party(state, auth()?, cmd)),
            Proc::DeleteParty(cmd) => todo!("DeleteParty"),
            Proc::TransferOwnership(cmd) => todo!("TransferOwnership"),
            Proc::CreateRole(cmd) => c!(party::roles::create_role::create_role(state, auth()?, cmd)),
            Proc::PatchRole(cmd) => c!(party::roles::modify_role::modify_role(state, auth()?, cmd)),
            Proc::DeleteRole(cmd) => todo!("DeleteRole"),
            Proc::GetPartyMembers(cmd) => s!(party::party_members::get_many(state, auth()?, cmd)),
            Proc::GetPartyMember(cmd) => c!(party::party_members::get_one(state, auth()?, cmd)),
            Proc::GetPartyRooms(cmd) => s!(party::rooms::get_rooms::get_party_rooms(state, auth()?, cmd)),
            Proc::GetPartyInvites(cmd) => todo!("GetPartyInvites"),
            Proc::GetMemberProfile(cmd) => todo!("GetMemberProfile"),
            Proc::UpdateMemberProfile(cmd) => c!(party::party_member_profile::patch_member_profile(state, auth()?, cmd)),
            Proc::CreatePartyInvite(cmd) => todo!("CreatePartyInvite"),
            Proc::CreatePinFolder(cmd) => todo!("CreatePinFolder"),
            Proc::CreateRoom(cmd) => c!(party::rooms::create_room::create_room(state, auth()?, cmd)),
            Proc::SearchParty(cmd) => todo!("SearchParty"),
            Proc::CreateMessage(cmd) => c!(room::messages::create_message::create_message(state, auth()?, cmd)),
            Proc::EditMessage(cmd) => c!(room::messages::edit_message::edit_message(state, auth()?, cmd)),
            Proc::GetMessage(cmd) => todo!("GetMessage"),
            Proc::DeleteMessage(cmd) => c!(room::messages::delete_message::delete_msg(state, auth()?, cmd)),
            Proc::StartTyping(cmd) => c!(room::start_typing::trigger_typing(state, auth()?, cmd)),
            Proc::GetMessages(cmd) => s!([1024] room::messages::get_messages::get_many(state, auth()?, cmd)),
            Proc::PinMessage(cmd) => todo!("PinMessage"),
            Proc::UnpinMessage(cmd) => todo!("UnpinMessage"),
            Proc::StarMessage(cmd) => todo!("StarMessage"),
            Proc::UnstarMessage(cmd) => todo!("UnstarMessage"),
            Proc::PutReaction(cmd) => c!(room::messages::reactions::add_reaction::add_reaction(state, auth()?, cmd)),
            Proc::DeleteOwnReaction(cmd) => c!(room::messages::reactions::remove_reaction::remove_own_reaction(state, auth()?, cmd)),
            Proc::DeleteUserReaction(cmd) => todo!("DeleteUserReaction"),
            Proc::DeleteAllReactions(cmd) => todo!("DeleteAllReactions"),
            Proc::GetReactions(cmd) => todo!("GetReactions"),
            Proc::PatchRoom(cmd) => c!(room::modify_room::modify_room(state, auth()?, cmd)),
            Proc::DeleteRoom(cmd) => c!(room::remove_room::remove_room(state, auth()?, cmd)),
            Proc::GetRoom(cmd) => c!(room::get_room::get_room(state, auth()?, cmd)),
        };

        Ok(running)
    };

    gen_dispatch()?.await
}
