use crate::prelude::*;

use rpc::request::ArchivedRpcRequest;
use tokio::io::AsyncWrite;

pub async fn dispatch<W>(state: ServerState, out: W, cmd: &ArchivedRpcRequest) -> Result<(), Error>
where
    W: AsyncWrite + Unpin + Send,
{
    use crate::api::party::{members::MemberMode, rooms::get::RoomScope};

    // avoid inlining every async state machine by boxing them inside a lazy future/async block
    macro_rules! c {
        ($([$size:literal])? $first:ident$(::$frag:ident)+($($args:expr),*)) => {
            Box::pin(async move { ::rpc::stream::encode_item::<_, Error, _, {512 $(* 0 + $size)?}>(
                out, crate::api::$first$(::$frag)+($($args),*).await).await.map_err(Error::from) })
        };
    }
    macro_rules! s {
        ($([$size:literal])? $first:ident$(::$frag:ident)+($($args:expr),*)) => {
            Box::pin(async move { ::rpc::stream::encode_stream::<_, Error, _, {512 $(* 0 + $size)?}>(
                out, crate::api::$first$(::$frag)+($($args),*).await).await.map_err(Error::from) })
        };
    }

    let ArchivedRpcRequest::Procedure { addr, auth, proc } = cmd else {
        unimplemented!();
    };

    // prepare fields
    let addr = addr.as_socket_addr();
    let auth = || match auth.as_ref() {
        Some(auth) => Ok(simple_de::<Authorization>(auth)),
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
        Proc::GetUser(form) => c!(user::get::get_full_user(state, auth()?, form.user_id)),
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
        Proc::CreateRole(form) => c!(party::roles::create::create_role(state, auth()?, form.party_id, &form.body)),
        Proc::PatchRole(form) => c!(party::roles::modify::modify_role(state, auth()?, form.party_id, form.role_id, &form.body)),
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
        Proc::CreateMessage(form) => c!(room::messages::create::create_message(state, auth()?, form.room_id, &form.body)),
        Proc::EditMessage(form) => c!(room::messages::edit::edit_message(state, auth()?, form.room_id, form.msg_id, &form.body)),
        Proc::GetMessage(form) => todo!("GetMessage"),
        Proc::DeleteMessage(form) => c!(room::messages::delete::delete_msg(state, auth()?, form.room_id, form.msg_id)),
        Proc::StartTyping(form) => c!(room::typing::trigger_typing(state, auth()?, form.room_id, &form.body)),
        Proc::GetMessages(form) => s!([1024] room::messages::get::get_many(state, auth()?, form.room_id, &form.body)),
        Proc::PinMessage(form) => todo!("PinMessage"),
        Proc::UnpinMessage(form) => todo!("UnpinMessage"),
        Proc::StarMessage(form) => todo!("StarMessage"),
        Proc::UnstarMessage(form) => todo!("UnstarMessage"),
        Proc::PutReaction(form) => c!(room::messages::reaction::add::add_reaction(state, auth()?, form.room_id, form.msg_id, &form.emote_id)),
        Proc::DeleteOwnReaction(form) => c!(room::messages::reaction::remove::remove_own_reaction(state, auth()?, form.room_id, form.msg_id, &form.emote_id)),
        Proc::DeleteUserReaction(form) => todo!("DeleteUserReaction"),
        Proc::DeleteAllReactions(form) => todo!("DeleteAllReactions"),
        Proc::GetReactions(form) => todo!("GetReactions"),
        Proc::PatchRoom(form) => c!(room::modify::modify_room(state, auth()?, form.room_id, &form.body)),
        Proc::DeleteRoom(form) => c!(room::remove::remove_room(state, auth()?, form.room_id)),
        Proc::GetRoom(form) => c!(room::get::get_room(state, auth()?, form.room_id)),
    };

    running.await
}
