use std::net::SocketAddr;

use sdk::api::commands::all::*;

#[derive(Debug, rkyv::Archive, rkyv::Serialize)]
pub struct Message {
    pub proc: Procedure,

    pub addr: SocketAddr,

    #[with(rkyv::with::Niche)]
    pub auth: Option<Box<crate::auth::Authorization>>,
}

const fn mirror_tag(t: u16) -> u32 {
    let le = t.to_le_bytes();
    let be = t.to_be_bytes();
    u32::from_le_bytes([le[0], le[1], be[0], be[1]])
}

macro_rules! decl_procs {
    ($($code:literal = $cmd:ident),*$(,)?) => {
        #[derive(Debug)]
        #[repr(u16)]
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

        #[cfg(test)]
        mod decl_procs_tests {
            use super::*;

            #[test]
            fn test_mirror_tag() {$(
                println!("0x{:04X} -> 0x{:08X} <-> 0x{:08X}", $code, mirror_tag($code), mirror_tag($code).swap_bytes());
                assert_eq!(mirror_tag($code), mirror_tag($code).swap_bytes());
            )*}
        }

        pub use proc_impl::{ArchivedProcedure, ProcedureResolver};

        mod proc_impl {paste::paste! {
            use super::*;

            #[derive(Debug, Clone, Copy, PartialEq, Eq)]
            #[repr(u32)]
            enum ArchivedTag {
                $($cmd = mirror_tag($code),)*
            }

            #[repr(u32)]
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
    402 = CreateParty,
    403 = PatchParty,
    404 = DeleteParty,
    405 = TransferOwnership,
    406 = CreateRole,
    407 = PatchRole,
    408 = DeleteRole,
    409 = GetPartyMembers,
    410 = GetPartyMember,
    411 = GetPartyRooms,
    412 = GetPartyInvites,
    413 = GetMemberProfile,
    414 = UpdateMemberProfile,
    415 = CreatePartyInvite,
    416 = CreatePinFolder,
    417 = CreateRoom,
    418 = SearchParty,

    501 = CreateMessage,
    502 = EditMessage,
    503 = GetMessage,
    504 = DeleteMessage,
    505 = StartTyping,
    506 = GetMessages,
    507 = PinMessage,
    508 = UnpinMessage,
    509 = StarMessage,
    510 = UnstarMessage,
    511 = PutReaction,
    512 = DeleteOwnReaction,
    513 = DeleteUserReaction,
    514 = DeleteAllReactions,
    515 = GetReactions,
    516 = PatchRoom,
    517 = DeleteRoom,
    518 = GetRoom,
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
