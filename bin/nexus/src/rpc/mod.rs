#![allow(clippy::redundant_closure)]

//pub mod admin;
//pub mod auth;
pub mod perm;

#[derive(Debug, Clone, Copy)]
pub enum SearchMode<'a> {
    Single(schema::Snowflake),
    Many(&'a [schema::Snowflake]),
}

pub mod user {
    pub mod user_get;
    pub mod user_login;
    pub mod user_register;

    pub mod me {
        pub mod user_account;
        pub mod user_change_password;

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
    use crate::rpc::party::{members::MemberMode, rooms::get::RoomScope};

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

    let ArchivedRpcRequest::Procedure { addr, auth, proc } = cmd else {
        unimplemented!();
    };

    // prepare fields
    let addr = addr.as_socket_addr();
    let auth = || match auth.as_ref() {
        Some(auth) => Ok(auth.simple_deserialize().expect("Unable to deserialize auth")),
        None => Err(Error::Unauthorized),
    };

    use core::{future::Future, pin::Pin};
    use rpc::procedure::ArchivedProcedure as Proc;

    #[allow(unused_variables)]
    #[rustfmt::skip]
    let running: Pin<Box<dyn Future<Output = Result<(), Error>> + Send>> = match proc {
        Proc::GetServerConfig(form) => todo!("GetServerConfig"),
        Proc::UserRegister(form) => c!(user::register::register_user(state, addr, &form.body)),
        Proc::UserLogin(form) => c!(user::me::login::login(state, addr, &form.body)),
        Proc::UserLogout(_) => c!(user::me::logout::logout_user(state, auth()?)),
        Proc::Enable2FA(form) => c!(user::me::mfa::enable_2fa(state, auth()?.user_id(), &form.body)),
        Proc::Confirm2FA(form) => c!(user::me::mfa::confirm_2fa(state, auth()?.user_id(), &form.body)),
        Proc::Remove2FA(form) => c!(user::me::mfa::remove_2fa(state, auth()?.user_id(), &form.body)),
        Proc::ChangePassword(form) => todo!("ChangePassword"),
        Proc::GetSessions(form) => s!(user::me::sessions::list_sessions(state, auth()?)),
        Proc::ClearSessions(form) => c!(user::me::sessions::clear_other_sessions(state, auth()?)),
        Proc::GetRelationships(form) => todo!("GetRelationships"),
        Proc::PatchRelationship(form) => todo!("PatchRelationship"),
        Proc::UpdateUserProfile(form) => c!(user::me::profile::patch_profile(state, auth()?, None, &form.body)),
        Proc::GetUser(form) => c!(user::get::get_full_user(state, auth()?, form)),
        Proc::UpdateUserPrefs(form) => c!(user::me::prefs::update_prefs(state, auth()?, &form.body.inner)),
        Proc::CreateFile(form) => todo!("CreateFile"),
        Proc::GetFilesystemStatus(form) => todo!("GetFilesystemStatus"),
        Proc::GetFileStatus(form) => todo!("GetFileStatus"),
        Proc::GetInvite(form) => todo!("GetInvite"),
        Proc::RevokeInvite(form) => todo!("RevokeInvite"),
        Proc::RedeemInvite(form) => todo!("RedeemInvite"),
        Proc::CreateParty(form) => c!(party::create::create_party(state, auth()?, &form.body)),
        Proc::GetParty(form) => c!(party::get::get_party(state, auth()?, form.party_id)),
        Proc::PatchParty(form) => c!(party::modify::modify_party(state, auth()?, form.party_id, &form.body)),
        Proc::DeleteParty(form) => todo!("DeleteParty"),
        Proc::TransferOwnership(form) => todo!("TransferOwnership"),
        Proc::CreateRole(form) => c!(party::roles::create_role::create_role(state, auth()?, form.party_id, &form.body)),
        Proc::PatchRole(form) => c!(party::roles::modify_role::modify_role(state, auth()?, form.party_id, form.role_id, &form.body)),
        Proc::DeleteRole(form) => todo!("DeleteRole"),
        Proc::GetPartyMembers(form) => s!(party::members::get_many(state, auth()?, form.party_id)),
        Proc::GetPartyMember(form) => c!(party::members::get_one(state, auth()?.user_id(), form.party_id, form.member_id, MemberMode::Full)),
        Proc::GetPartyRooms(form) => s!(party::rooms::get::get_rooms(state, auth()?, RoomScope::Party(form.party_id))),
        Proc::GetPartyInvites(form) => todo!("GetPartyInvites"),
        Proc::GetMemberProfile(form) => todo!("GetMemberProfile"),
        Proc::UpdateMemberProfile(form) => c!(user::me::profile::patch_profile(state, auth()?, Some(form.party_id), &form.body.profile)),
        Proc::CreatePartyInvite(form) => todo!("CreatePartyInvite"),
        Proc::CreatePinFolder(form) => todo!("CreatePinFolder"),
        Proc::CreateRoom(form) => c!(party::rooms::create::create_room(state, auth()?, form.party_id, &form.body)),
        Proc::SearchParty(form) => todo!("SearchParty"),
        Proc::CreateMessage(form) => c!(room::messages::create_message::create_message(state, auth()?, form.room_id, &form.body)),
        Proc::EditMessage(form) => c!(room::messages::edit_message::edit_message(state, auth()?, form.room_id, form.msg_id, &form.body)),
        Proc::GetMessage(form) => todo!("GetMessage"),
        Proc::DeleteMessage(form) => c!(room::messages::delete_message::delete_msg(state, auth()?, form.room_id, form.msg_id)),
        Proc::StartTyping(form) => c!(room::start_typing::trigger_typing(state, auth()?, form.room_id, &form.body)),
        Proc::GetMessages(form) => s!([1024] room::messages::get_messages::get_many(state, auth()?, form.room_id, &form.body)),
        Proc::PinMessage(form) => todo!("PinMessage"),
        Proc::UnpinMessage(form) => todo!("UnpinMessage"),
        Proc::StarMessage(form) => todo!("StarMessage"),
        Proc::UnstarMessage(form) => todo!("UnstarMessage"),
        Proc::PutReaction(form) => c!(room::messages::reaction::add_reaction::add_reaction(state, auth()?, form.room_id, form.msg_id, &form.emote_id)),
        Proc::DeleteOwnReaction(form) => c!(room::messages::reaction::remove_reaction::remove_own_reaction(state, auth()?, form.room_id, form.msg_id, &form.emote_id)),
        Proc::DeleteUserReaction(form) => todo!("DeleteUserReaction"),
        Proc::DeleteAllReactions(form) => todo!("DeleteAllReactions"),
        Proc::GetReactions(form) => todo!("GetReactions"),
        Proc::PatchRoom(form) => c!(room::modify_room::modify_room(state, auth()?, form.room_id, &form.body)),
        Proc::DeleteRoom(form) => c!(room::remove_room::remove_room(state, auth()?, form.room_id)),
        Proc::GetRoom(form) => c!(room::get_room::get_room(state, auth()?, form.room_id)),
    };

    running.await
}
