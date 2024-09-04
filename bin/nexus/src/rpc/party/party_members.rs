use crate::prelude::*;

use sdk::{
    api::commands::all::{GetPartyMember, GetPartyMembers},
    models::*,
};

use crate::internal::get_members::{get_members, MemberMode};

pub async fn get_one(
    state: ServerState,
    auth: Authorization,
    cmd: &Archived<GetPartyMember>,
) -> Result<PartyMember, Error> {
    let party_id: PartyId = cmd.party_id.into();
    let user_id: UserId = auth.user_id();
    let member_id: UserId = cmd.member_id.into();

    let mut stream =
        std::pin::pin!(get_members(state, party_id, Some(user_id), Some(member_id), MemberMode::Full).await?);

    match stream.next().await {
        Some(first) => first,
        None => Err(Error::NotFound),
    }
}

pub async fn get_many(
    state: ServerState,
    auth: Authorization,
    cmd: &Archived<GetPartyMembers>,
) -> Result<impl Stream<Item = Result<PartyMember, Error>>, Error> {
    let party_id: PartyId = cmd.party_id.into();

    get_members(state, party_id, Some(auth.user_id()), None, MemberMode::Simple).await
}
