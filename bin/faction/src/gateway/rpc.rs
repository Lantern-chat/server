use crate::prelude::*;

use futures::{Stream, StreamExt};
use tokio::io::{AsyncWrite, AsyncWriteExt};

use framed::tokio::AsyncFramedWriter;

use rkyv::ser::serializers::AllocSerializer;
use rkyv::{ser::Serializer, Serialize};

async fn encode_item<T, W, const N: usize>(mut out: AsyncFramedWriter<W>, item: T) -> Result<(), Error>
where
    W: AsyncWrite + Unpin,
    T: Serialize<AllocSerializer<N>>,
{
    let mut serializer = AllocSerializer::<N>::default();

    if let Err(e) = serializer.serialize_value(&item) {
        log::error!("Rkyv Error: {e}");
        return Err(Error::RkyvEncodingError);
    }

    let archived = serializer.into_serializer().into_inner();

    out.write_msg(archived.as_slice()).await?;

    Ok(())
}

async fn encode_stream<T, W, const N: usize>(
    mut out: AsyncFramedWriter<W>,
    stream: impl Stream<Item = Result<T, Error>>,
) -> Result<(), Error>
where
    W: AsyncWrite + Unpin,
    T: Serialize<AllocSerializer<N>>,
{
    let mut serializer = AllocSerializer::default();

    let mut stream = std::pin::pin!(stream);
    while let Some(item) = stream.next().await {
        serializer.serialize_value(&item?);
        let mut msg = out.new_message();
        msg.write_all(serializer.serializer().inner().as_slice()).await?;
        serializer.reset(); // immediately free buffers before flushing
        AsyncFramedWriter::dispose_msg(msg).await?;
    }

    Ok(())
}

pub async fn dispatch<W>(
    state: ServerState,
    out: AsyncFramedWriter<W>,
    msg: &rpc::msg::ArchivedMessage,
) -> Result<(), Error>
where
    W: AsyncWrite + Unpin,
{
    use crate::api::party::{members::MemberMode, rooms::get::RoomScope};

    // avoid inlining every async state machine by boxing them inside a lazy future/async block
    macro_rules! c {
        ($first:ident$(::$frag:ident)+($($args:expr),*)) => {
            Box::pin(async move { encode_item(out, crate::api::$first$(::$frag)+($($args),*).await?).await })
        };
    }
    macro_rules! s {
        ($first:ident$(::$frag:ident)+($($args:expr),*)) => {
            Box::pin(async move { encode_stream(out, crate::api::$first$(::$frag)+($($args),*).await?).await })
        };
    }

    // prepare fields
    let addr = msg.addr.as_socket_addr();
    let auth = || match msg.auth.as_deref() {
        Some(auth) => Ok(simple_de::<Authorization>(auth)),
        None => Err(Error::Unauthorized),
    };

    use core::{future::Future, pin::Pin};
    use rpc::msg::ArchivedProcedure as Proc;

    #[rustfmt::skip]
    let running: Pin<Box<dyn Future<Output = Result<(), Error>>>> = match &msg.proc {
        Proc::GetServerConfig(form) => todo!("GetServerConfig"),
        Proc::UserRegister(form) => c!(user::register::register_user(state, addr, &form.body)),
        Proc::UserLogin(form) => c!(user::me::login::login(state, addr, &form.body)),
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
        Proc::GetMessages(form) => s!(room::messages::get::get_many(state, auth()?, form.room_id, &form.body)),
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
