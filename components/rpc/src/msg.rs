use sdk::api::commands::all::*;

#[derive(Debug, rkyv::Archive, rkyv::Serialize)]
pub struct Message {
    pub proc: Procedure,

    #[with(rkyv::with::Niche)]
    pub auth: Option<Box<crate::auth::Authorization>>,
}

macro_rules! decl_procs {
    (#[repr($repr:ty)] { $($code:literal = $cmd:ident),*$(,)? }) => {
        #[derive(Debug)]
        #[repr($repr)]
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

        #[repr($repr)]
        pub enum ArchivedProcedure {
            $($cmd(rkyv::Archived<Box<$cmd>>) = $code,)*
        }

        #[repr($repr)]
        pub enum ProcedureResolver {
            $($cmd(rkyv::Resolver<Box<$cmd>>) = $code,)*
        }

        const _: () = {paste::paste! {
            use core::marker::PhantomData;
            use rkyv::{Archive, Archived, Serialize, Fallible, Deserialize};

            #[derive(Debug, Clone, Copy, PartialEq, Eq)]
            #[repr($repr)]
            enum Tag {
                $($cmd = $code,)*
            }

            $(
                #[repr(C)]
                struct [<Archived $cmd Variant>] {
                    tag: Tag,
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
                                core::ptr::addr_of_mut!((*out).tag).write(Tag::$cmd);
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

            #[allow(non_upper_case_globals)]
            mod discriminant {
                $(pub const $cmd: $repr = $code;)*
            }

            impl<C: ?Sized> CheckBytes<C> for ArchivedProcedure
                where $(Archived<Box<$cmd>>: CheckBytes<C>,)*
            {
                type Error = EnumCheckError<$repr>;

                unsafe fn check_bytes<'a>(value: *const Self, context: &mut C) -> Result<&'a Self, EnumCheckError<$repr>> {
                    let tag = *value.cast::<$repr>();

                    match tag {
                    $(
                        discriminant::$cmd => {
                            let value = value.cast::<[<Archived $cmd Variant>]>();

                            if let Err(e) = <Archived<Box<$cmd>> as CheckBytes<C>>::check_bytes(core::ptr::addr_of!((*value).cmd), context) {
                                return Err(EnumCheckError::InvalidTuple {
                                    variant_name: stringify!($cmd),
                                    inner: TupleStructCheckError {
                                        field_index: 0,
                                        inner: ErrorBox::new(e),
                                    }
                                });
                            }
                        }
                    )*
                        _ => return Err(EnumCheckError::InvalidTag(tag)),
                    }

                    Ok(&*value)
                }
            }
        }};
    };
}

decl_procs! { #[repr(u16)] {
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
}}

#[cfg(test)]
mod tests {
    use rkyv::Deserialize;

    use super::*;

    #[test]
    fn test_rkyv() {
        let p = rkyv::to_bytes::<_, 256>(&Procedure::from(GetServerConfig::new())).unwrap();
        let a = rkyv::check_archived_root::<Procedure>(&p).unwrap();
        let _: Procedure = a.deserialize(&mut rkyv::Infallible).unwrap();
    }
}
