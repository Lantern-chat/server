use crate::prelude::*;

#[rustfmt::skip]
pub async fn dispatch(state: ServerState, msg: &rpc::msg::ArchivedMessage) -> Result<rkyv::AlignedVec, Error> {
    macro_rules! c {
        ($first:ident$(::$frag:ident)+($($args:expr),*)) => {
            // TODO: Add error for rkyv encoding instead of unwrapping
            Box::pin(async move { Ok(rkyv::util::to_bytes::<_, 1024>(&crate::api::$first$(::$frag)+($($args),*).await?).unwrap()) }).await
        };
    }

    // prepare fields
    let auth = msg.auth.as_deref().copied().ok_or(Error::Unauthorized);
    let addr = msg.addr.as_socket_addr();

    use rpc::msg::ArchivedProcedure as Proc;

    match &msg.proc {
        Proc::GetServerConfig(_) => todo!("GetServerConfig"),
        Proc::UserRegister(form) => c!(user::register::register_user(state, addr, &form.body)),
        Proc::UserLogin(form) => c!(user::me::login::login(state, addr, &form.body)),
        Proc::Enable2FA(_) => todo!("Enable2FA"),
        Proc::Confirm2FA(form) => c!(user::me::mfa::confirm_2fa(state, auth?.user_id(), &form.body)),
        Proc::Remove2FA(_) => todo!("Remove2FA"),
        Proc::ChangePassword(_) => todo!("ChangePassword"),
        Proc::GetSessions(_) => todo!("GetSessions"),
        Proc::ClearSessions(_) => todo!("ClearSessions"),
        Proc::GetRelationships(_) => todo!("GetRelationships"),
        Proc::PatchRelationship(_) => todo!("PatchRelationship"),
        Proc::UpdateUserProfile(_) => todo!("UpdateUserProfile"),
        Proc::GetUser(form) => c!(user::get::get_full_user(state, auth?, form.user_id)),
        Proc::UpdateUserPrefs(form) => c!(user::me::prefs::update_prefs(state, auth?, &form.body.inner)),
        Proc::CreateFile(_) => todo!("CreateFile"),
        Proc::GetFilesystemStatus(_) => todo!("GetFilesystemStatus"),
        Proc::GetFileStatus(_) => todo!("GetFileStatus"),
        Proc::GetInvite(_) => todo!("GetInvite"),
        Proc::RevokeInvite(_) => todo!("RevokeInvite"),
        Proc::RedeemInvite(_) => todo!("RedeemInvite"),
        Proc::CreateParty(form) => c!(party::create::create_party(state, auth?, &form.body)),
        Proc::GetParty(_) => todo!("GetParty"),
        Proc::PatchParty(form) => c!(party::modify::modify_party(state, auth?, form.party_id, &form.body)),
        Proc::DeleteParty(_) => todo!("DeleteParty"),
        Proc::TransferOwnership(_) => todo!("TransferOwnership"),
        Proc::CreateRole(form) => c!(party::roles::create::create_role(state, auth?, form.party_id, &form.body)),
        Proc::PatchRole(_) => todo!("PatchRole"),
        Proc::DeleteRole(_) => todo!("DeleteRole"),
        Proc::GetPartyMembers(_) => todo!("GetPartyMembers"),
        Proc::GetPartyRooms(_) => todo!("GetPartyRooms"),
        Proc::GetPartyInvites(_) => todo!("GetPartyInvites"),
        Proc::GetMemberProfile(_) => todo!("GetMemberProfile"),
        Proc::UpdateMemberProfile(_) => todo!("UpdateMemberProfile"),
        Proc::CreatePartyInvite(_) => todo!("CreatePartyInvite"),
        Proc::CreatePinFolder(_) => todo!("CreatePinFolder"),
        Proc::CreateRoom(_) => todo!("CreateRoom"),
        Proc::SearchParty(_) => todo!("SearchParty"),
        Proc::CreateMessage(_) => todo!("CreateMessage"),
        Proc::EditMessage(_) => todo!("EditMessage"),
        Proc::GetMessage(_) => todo!("GetMessage"),
        Proc::StartTyping(_) => todo!("StartTyping"),
        Proc::GetMessages(_) => todo!("GetMessages"),
        Proc::PinMessage(_) => todo!("PinMessage"),
        Proc::UnpinMessage(_) => todo!("UnpinMessage"),
        Proc::StarMessage(_) => todo!("StarMessage"),
        Proc::UnstarMessage(_) => todo!("UnstarMessage"),
        Proc::PutReaction(_) => todo!("PutReaction"),
        Proc::DeleteOwnReaction(_) => todo!("DeleteOwnReaction"),
        Proc::DeleteUserReaction(_) => todo!("DeleteUserReaction"),
        Proc::DeleteAllReactions(_) => todo!("DeleteAllReactions"),
        Proc::GetReactions(_) => todo!("GetReactions"),
        Proc::PatchRoom(_) => todo!("PatchRoom"),
    }
}
