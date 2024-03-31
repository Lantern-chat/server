use sdk::api::commands::all::*;

use crate::client::Resolve;

// Note to self: figure out a way to send the party/room lookups via the RPC messages

const fn mirror_tag(t: u16) -> u32 {
    let le = t.to_le_bytes();
    u32::from_le_bytes([le[0], le[1], le[1], le[0]])
}

macro_rules! decl_procs {
    ($($code:literal = $cmd:ident $(@ $kind:ident $(.$path:ident)+)?),*$(,)?) => {
        pub use proc_impl::{ArchivedProcedure, ProcedureResolver};

        #[derive(Debug)]
        #[repr(u16)]
        pub enum Procedure {
            $($cmd(Box<$cmd>) = $code,)*
        }

        impl Procedure {
            pub fn endpoint(&self) -> Resolve {
                match self {
                    $(Self::$cmd(_cmd) => Resolve::Nexus $(.$kind(_cmd $(.$path)+))?),*
                }
            }
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

        #[cfg(test)]
        mod decl_procs_tests {
            use super::*;

            #[test]
            fn test_mirror_tag() {$(
                println!("0x{:04X} -> 0x{:08X} <-> 0x{:08X}", $code, mirror_tag($code), mirror_tag($code).swap_bytes());
                assert_eq!(mirror_tag($code), mirror_tag($code).swap_bytes());
            )*}
        }

        mod proc_impl {paste::paste! {
            use super::*;

            #[derive(Debug, Clone, Copy, PartialEq, Eq)]
            #[repr(u32, align(4))]
            enum ArchivedTag {
                $($cmd = mirror_tag($code),)*
            }

            #[repr(u32, align(4))]
            pub enum ArchivedProcedure {
                $($cmd(rkyv::Archived<Box<$cmd>>) = ArchivedTag::$cmd as u32,)*
            }

            #[repr(u16)]
            pub enum ProcedureResolver {
                $($cmd(rkyv::Resolver<Box<$cmd>>) = $code,)*
            }

            use core::marker::PhantomData;
            use rkyv::{Archive, Archived, Serialize, Fallible, Deserialize};

            $(
                #[repr(C)]
                struct [<Archived $cmd Variant>] {
                    tag: ArchivedTag,
                    cmd: Archived<Box<$cmd>>,
                    mkr: PhantomData<Procedure>,
                }
            )*

            impl Archive for Procedure {
                type Archived = ArchivedProcedure;
                type Resolver = ProcedureResolver;

                unsafe fn resolve(&self, pos: usize, resolver: ProcedureResolver, out: *mut ArchivedProcedure) {
                    match resolver {$(
                        ProcedureResolver::$cmd(resolver_0) => match self {
                            Procedure::$cmd(self_0) => {
                                let out = out.cast::<[<Archived $cmd Variant>]>();
                                core::ptr::addr_of_mut!((*out).tag).write(ArchivedTag::$cmd);
                                let (fp, fo) = rkyv::out_field!(out.cmd);
                                rkyv::Archive::resolve(self_0, pos + fp, resolver_0, fo);
                            },
                            _ => core::hint::unreachable_unchecked(),
                        },
                    )*}
                }
            }

            impl<S: Fallible + ?Sized> Serialize<S> for Procedure
                where $(Box<$cmd>: Serialize<S>,)*
            {
                fn serialize(&self, serializer: &mut S) -> Result<ProcedureResolver, S::Error> {
                    Ok(match self {
                        $(Procedure::$cmd(cmd) => ProcedureResolver::$cmd(Serialize::serialize(cmd, serializer)?),)*
                    })
                }
            }

            impl<D: Fallible + ?Sized> Deserialize<Procedure, D> for ArchivedProcedure
                where $(Archived<Box<$cmd>>: Deserialize<Box<$cmd>, D>,)*
            {
                fn deserialize(&self, deserializer: &mut D) -> Result<Procedure, D::Error> {
                    Ok(match self {$(
                        ArchivedProcedure::$cmd(cmd) => Procedure::$cmd(Deserialize::deserialize(cmd, deserializer)?),
                    )*})
                }
            }

            use rkyv::bytecheck::{CheckBytes, EnumCheckError, ErrorBox, TupleStructCheckError};

            impl<C: ?Sized> CheckBytes<C> for ArchivedProcedure
                where $(Archived<Box<$cmd>>: CheckBytes<C>,)*
            {
                type Error = EnumCheckError<u32>;

                unsafe fn check_bytes<'a>(value: *const Self, context: &mut C) -> Result<&'a Self, Self::Error> {
                    let tag = *value.cast::<u32>();

                    struct Discriminant;

                    #[allow(non_upper_case_globals)]
                    impl Discriminant {
                        $(pub const $cmd: u32 = ArchivedTag::$cmd as u32;)*
                    }

                    match tag {
                    $(
                        Discriminant::$cmd => {
                            let value = value.cast::<[<Archived $cmd Variant>]>();

                            if let Err(e) = <Archived<Box<$cmd>> as CheckBytes<C>>::check_bytes(core::ptr::addr_of!((*value).cmd), context) {
                                return Err(EnumCheckError::InvalidTuple {
                                    variant_name: stringify!($cmd),
                                    inner: TupleStructCheckError { field_index: 0, inner: ErrorBox::new(e) }
                                });
                            }
                        }
                    )*
                        _ => return Err(EnumCheckError::InvalidTag(tag)),
                    }

                    Ok(&*value)
                }
            }
        }}
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

#[cfg(test)]
mod tests {
    use rkyv::Deserialize;

    use super::*;

    #[test]
    fn test_rkyv() {
        let p = rkyv::to_bytes::<_, 256>(&Procedure::from(GetServerConfig::new())).unwrap();
        let a = rkyv::check_archived_root::<Procedure>(&p).unwrap();
        let Procedure::GetServerConfig(_) = a.deserialize(&mut rkyv::Infallible).unwrap() else {
            panic!("Wrong variant");
        };
    }
}
