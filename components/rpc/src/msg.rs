use sdk::api::commands::all::*;

#[derive(Debug, rkyv::Archive, rkyv::Serialize)]
pub struct Message {
    pub proc: Procedure,

    #[with(rkyv::with::Niche)]
    pub auth: Option<Box<crate::auth::Authorization>>,
}

macro_rules! decl_procs {
    ($($code:literal = $cmd:ident),*$(,)?) => {
        #[derive(Debug, rkyv::Archive, rkyv::Serialize)]
        #[repr(u32)]
        pub enum Procedure {
            $($cmd(Box<$cmd>) = $code,)*
        }

        $(
            impl From<$cmd> for Procedure {
                #[inline]
                fn from(cmd: $cmd) -> Procedure {
                    Procedure::$cmd(Box::new(cmd))
                }
            }

            impl From<Box<$cmd>> for Procedure {
                #[inline]
                fn from(cmd: Box<$cmd>) -> Procedure {
                    Procedure::$cmd(cmd)
                }
            }
        )*
    };
}

decl_procs! {
    0   = GetServerConfig,

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

    201 = CreateFile,
    202 = GetFilesystemStatus,
    203 = GetFileStatus,

    301 = GetInvite,
    302 = RevokeInvite,
    303 = RedeemInvite,

    401 = GetParty,
    402 = PatchParty,
    403 = DeleteParty,
    404 = TransferOwnership,
    405 = CreateRole,
    406 = PatchRole,
    407 = DeleteRole,
    408 = GetPartyMembers,
    409 = GetPartyRooms,
    410 = GetPartyInvites,
    411 = GetMemberProfile,
    412 = UpdateMemberProfile,
    413 = CreatePartyInvite,
    414 = CreatePinFolder,
    415 = CreateRoom,
    416 = SearchParty,

    501 = CreateMessage,
    502 = EditMessage,
    503 = GetMessage,
    504 = StartTyping,
    505 = GetMessages,
    506 = PinMessage,
    507 = UnpinMessage,
    508 = StarMessage,
    509 = UnstarMessage,
    510 = PutReaction,
    511 = DeleteOwnReaction,
    512 = DeleteUserReaction,
    513 = DeleteAllReactions,
    514 = GetReactions,
    515 = PatchRoom,
}
