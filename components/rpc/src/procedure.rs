#![allow(unused_imports)]

use crate::client::Resolve;
use sdk::api::commands::all::*;

macro_rules! decl_procs {
    ($($code:literal = $cmd:ident $(@ $kind:ident $(.$path:ident)+)?),*$(,)?) => {
        rkyv_rpc::tuple_enum! {
            #[derive(Debug)]
            pub enum Box<Procedure>: u16 {
                $($code = $cmd($cmd),)*
            }
        }

        impl Procedure {
            pub fn endpoint(&self) -> Resolve {
                match self {
                    $(Self::$cmd(_cmd) => Resolve::Nexus $(.$kind(_cmd $(.$path)+))?),*
                }
            }

            #[cfg(feature = "ftl")]
            pub fn stream_response<S, E>(&self, recv: S, encoding: Encoding) -> BoxFuture<Result<ftl::Response, E>>
            where
                S: Send + AsyncRead + Unpin + 'static,
                E: From<std::io::Error> + From<ApiError> + 'static,
            {
                match self {
                    $(Self::$cmd(_) => stream_response::<_, $cmd, _, _>(recv, encoding).boxed()),*
                }
            }
        }
    };
}

decl_procs! {
    // Stuff not actually sent to a backend
    0   = GetServerConfig,

    // User stuff, all goes to the Nexus
    101 = UserRegister,
    102 = UserLogin,
    103 = Enable2FA,
    104 = Confirm2FA,
    105 = Remove2FA,
    106 = ChangePassword,
    107 = GetSessions,
    108 = ClearSessions,
    109 = GetRelationships,
    110 = PatchRelationship,
    111 = UpdateUserProfile,
    112 = GetUser,
    113 = UpdateUserPrefs,
    114 = UserLogout,

    // File stuff, will either go to the Nexus or CDN nodes
    201 = CreateFile,
    202 = GetFilesystemStatus,
    203 = GetFileStatus,

    // Invite stuff, goes to the nexus first
    301 = GetInvite,
    302 = RevokeInvite,
    303 = RedeemInvite,

    // Party stuff, goes to faction servers
    401 = CreateParty,
    402 = GetParty              @ party.party_id,
    403 = PatchParty            @ party.party_id,
    404 = DeleteParty           @ party.party_id,
    405 = TransferOwnership     @ party.party_id,
    406 = CreateRole            @ party.party_id,
    407 = PatchRole             @ party.party_id,
    408 = DeleteRole            @ party.party_id,
    409 = GetPartyMembers       @ party.party_id,
    410 = GetPartyMember        @ party.party_id,
    411 = GetPartyRooms         @ party.party_id,
    412 = GetPartyInvites       @ party.party_id,
    413 = GetMemberProfile      @ party.party_id,
    414 = UpdateMemberProfile   @ party.party_id,
    415 = CreatePartyInvite     @ party.party_id,
    416 = CreatePinFolder       @ party.party_id,
    417 = CreateRoom            @ party.party_id,
    418 = SearchParty           @ party.party_id,

    // Room stuff, also goes to faction servers but needs a party_id lookup first
    501 = CreateMessage         @ room.room_id,
    502 = EditMessage           @ room.room_id,
    503 = GetMessage            @ room.room_id,
    504 = DeleteMessage         @ room.room_id,
    505 = StartTyping           @ room.room_id,
    506 = GetMessages           @ room.room_id,
    507 = PinMessage            @ room.room_id,
    508 = UnpinMessage          @ room.room_id,
    509 = StarMessage           @ room.room_id,
    510 = UnstarMessage         @ room.room_id,
    511 = PutReaction           @ room.room_id,
    512 = DeleteOwnReaction     @ room.room_id,
    513 = DeleteUserReaction    @ room.room_id,
    514 = DeleteAllReactions    @ room.room_id,
    515 = GetReactions          @ room.room_id,
    516 = PatchRoom             @ room.room_id,
    517 = DeleteRoom            @ room.room_id,
    518 = GetRoom               @ room.room_id,
}

use futures_util::{future::BoxFuture, FutureExt, StreamExt};

use sdk::api::error::ApiError;
use sdk::{api::Command, driver::Encoding};
use tokio::io::AsyncRead;

use crate::stream::RpcRecvReader;

#[cfg(feature = "ftl")]
use rkyv::{
    api::high::{HighDeserializer, HighValidator},
    bytecheck::CheckBytes,
    rancor::{Error as RancorError, Source, Strategy},
    Archive, Archived, Deserialize,
};

#[cfg(feature = "ftl")]
pub async fn stream_response<S, P, T, E>(recv: S, encoding: Encoding) -> Result<ftl::Response, E>
where
    S: Send + AsyncRead + Unpin + 'static,
    P: Command<Item = T>,
    T: 'static + serde::Serialize + Archive + Send + Sync,
    Archived<T>: Deserialize<T, HighDeserializer<RancorError>>,
    Archived<T>: for<'b> CheckBytes<HighValidator<'b, RancorError>>,
    E: From<std::io::Error> + From<ApiError> + 'static,
{
    use ftl::Reply;

    let mut stream = Box::pin(RpcRecvReader::new(recv).recv_stream_deserialized::<Result<T, ApiError>>());

    let Some(first) = stream.next().await else {
        return Err(std::io::Error::new(std::io::ErrorKind::UnexpectedEof, "No Response").into());
    };

    let first = first??;

    if !P::STREAM {
        return Ok(match encoding {
            Encoding::JSON => ftl::reply::json::json(first).into_response(),
            Encoding::CBOR => ftl::reply::cbor::cbor(first).into_response(),
        });
    }

    // put the first item back into the stream and merge additional errors into IO errors
    //
    // In practice, only the first item should be an API error.
    let stream = futures_util::stream::iter([Ok(first)]).chain(stream.map(|value| match value {
        Ok(Ok(item)) => Ok(item),
        Ok(Err(api_error)) => Err(std::io::Error::new(std::io::ErrorKind::Other, api_error)),
        Err(err) => Err(err),
    }));

    Ok(match encoding {
        Encoding::JSON => ftl::reply::json::array_stream(stream).into_response(),
        Encoding::CBOR => ftl::reply::cbor::array_stream(stream).into_response(),
    })
}

#[cfg(test)]
mod tests {
    use rkyv::{access, rancor::Error as RancorError, Deserialize};

    use super::*;

    #[test]
    fn test_rkyv() {
        let p = rkyv::to_bytes::<RancorError>(&Procedure::from(GetServerConfig::new())).unwrap();
        let a = rkyv::access::<Archived<Procedure>, RancorError>(&p).unwrap();

        let Procedure::GetServerConfig(_) = rkyv::deserialize::<_, RancorError>(a).unwrap() else {
            panic!("Wrong variant");
        };
    }
}
