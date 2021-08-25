use serde_repr::{Deserialize_repr, Serialize_repr};

use serde::de::{self, Deserialize, Deserializer, MapAccess, Visitor};

// Utility function where the
#[inline]
fn is_default<T>(value: &T) -> bool
where
    T: Default + Eq,
{
    *value == T::default()
}

macro_rules! decl_msgs {
    ($($code:expr => $opcode:ident $(:$Default:ident)? {
        $( $(#[$field_meta:meta])* $field:ident : $ty:ty),*$(,)?
    }),*$(,)?) => {paste::paste!{
        #[derive(Debug, Clone, Copy, Serialize_repr, Deserialize_repr)]
        #[repr(u8)]
        pub enum Opcode {
            $($opcode = $code,)*
        }

        pub mod payloads { use super::*; $(
            #[derive(Debug, Clone, Serialize, Deserialize)]
            $(#[derive($Default, PartialEq, Eq)])?
            pub struct [<$opcode Payload>] {
                $($(#[$field_meta])* pub $field : $ty,)*
            }
        )*}

        #[derive(Debug, Serialize)]
        #[serde(untagged)] // custom tagging
        pub enum Message {$(
            $opcode {
                #[serde(rename = "o")]
                op: Opcode,

                #[serde(rename = "p")]
                $(#[serde(skip_serializing_if = "" [< is_ $Default:lower >] "" )])?
                payload: payloads::[<$opcode Payload>],
            },)*
        }

        impl Message {
            $(
                pub const fn [<$opcode:lower>](payload: payloads::[<$opcode Payload>]) -> Message {
                    Message::$opcode { op: Opcode::$opcode, payload }
                }

                pub const fn [<new_ $opcode:lower>]($($field: $ty),*) -> Message {
                    Message::$opcode { op: Opcode::$opcode, payload: payloads::[<$opcode Payload>] { $($field),* }}
                }
            )*
        }

        impl<'de> Deserialize<'de> for Message {
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
            where
                D: Deserializer<'de>
            {
                use std::fmt;

                #[derive(Deserialize)]
                enum Field {
                    #[serde(rename = "o")]
                    Opcode,

                    #[serde(rename = "p")]
                    Payload,
                }

                struct MessageVisitor;

                impl<'de> Visitor<'de> for MessageVisitor {
                    type Value = Message;

                    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                        formatter.write_str("struct Message")
                    }

                    fn visit_map<V>(self, mut map: V) -> Result<Message, V::Error>
                    where
                        V: MapAccess<'de>,
                    {
                        let opcode = match map.next_entry()? {
                            Some((Field::Opcode, o)) => o,
                            _ => return Err(de::Error::custom("Missing opcode first")),
                        };

                        match opcode {
                            $(
                                Opcode::$opcode => Ok(Message::$opcode {
                                    op: opcode,
                                    payload: match map.next_entry()? {
                                        Some((Field::Payload, payload)) => payload,
                                        $(None => $Default::default(),)?
                                        _ => return Err(de::Error::missing_field("payload")),
                                    }
                                }),
                            )*
                            // _ => Err(de::Error::custom("Invalid opcode")),
                        }
                    }
                }

                deserializer.deserialize_struct("Message", &["o", "p"], MessageVisitor)
            }
        }
    }}
}

use schema::Snowflake;

pub type ClientMsg = client::Message;
pub type ServerMsg = server::Message;

pub mod server {
    use super::*;

    use std::sync::Arc;

    use models::{
        events::{Hello, Ready, TypingStart},
        Intent, Message as RoomMessage, User, UserPresence,
    };

    type Room = (); // TODO

    #[derive(Debug, Serialize, Deserialize)]
    pub struct UserPresenceInner {
        pub user: User,
        pub presence: UserPresence,
    }

    // TODO: Check that this enum doesn't grow too large, allocate large payloads like Ready
    decl_msgs! {
        0 => Hello { #[serde(flatten)] inner: Hello },

        1 => HeartbeatACK: Default {},
        2 => Ready { #[serde(flatten)] inner: Box<Ready> },
        3 => InvalidSession: Default {},

        4 => PartyCreate {},
        5 => PartyUpdate {},
        6 => PartyDelete {},

        7 => RoleCreate {},
        8 => RoleUpdate {},
        9 => RoleDelete {},

        10 => MemberAdd {},
        11 => MemberUpdate {},
        12 => MemberRemove {},

        13 => RoomCreate { #[serde(flatten)] room: Room },
        14 => RoomUpdate { #[serde(flatten)] room: Room },
        15 => RoomDelete { id: Snowflake },
        16 => RoomPinsUpdate {},

        17 => MessageCreate { #[serde(flatten)] msg: RoomMessage },
        18 => MessageUpdate { #[serde(flatten)] msg: RoomMessage },
        19 => MessageDelete { #[serde(flatten)] msg: RoomMessage },

        20 => MessageReactionAdd {},
        21 => MessageReactionRemove {},
        22 => MessageReactionRemoveAll {},
        23 => MessageReactionRemoveEmote {},

        24 => PresenceUpdate {
            party: Option<Snowflake>,
            #[serde(flatten)] inner: Arc<UserPresenceInner>,
        },
        25 => TypingStart { #[serde(flatten)] t: Box<TypingStart> },
        26 => UserUpdate { user: Arc<User> }
    }

    impl Message {
        #[rustfmt::skip]
        pub fn matching_intent(&self) -> Option<Intent> {
            Some(match *self {
                Message::PartyCreate { .. } |
                Message::PartyDelete { .. } |
                Message::PartyUpdate { .. } |
                Message::RoleCreate { .. } |
                Message::RoleDelete { .. } |
                Message::RoleUpdate { .. } |
                Message::RoomPinsUpdate { .. } |
                Message::RoomCreate { .. } |
                Message::RoomDelete { .. } |
                Message::RoomUpdate { .. } => Intent::PARTIES,

                Message::MemberAdd { .. } |
                Message::MemberRemove { .. } |
                Message::MemberUpdate { .. } => Intent::PARTY_MEMBERS,

                Message::MessageCreate { .. } |
                Message::MessageDelete { .. } |
                Message::MessageUpdate { .. } => Intent::MESSAGES,

                Message::MessageReactionAdd { .. } |
                Message::MessageReactionRemove { .. } |
                Message::MessageReactionRemoveAll { .. } |
                Message::MessageReactionRemoveEmote { .. } => Intent::MESSAGE_REACTIONS,

                Message::PresenceUpdate { .. } => Intent::PRESENCE,
                Message::TypingStart { .. } => Intent::MESSAGE_TYPING,

                Message::Hello { .. } |
                Message::HeartbeatACK { .. } |
                Message::Ready { .. } |
                Message::InvalidSession { .. } |
                Message::UserUpdate { .. } => return None,
            })
        }
    }
}

pub mod client {
    use super::*;

    use models::{
        commands::{Identify, SetPresence},
        Intent,
    };

    decl_msgs! {
        0 => Heartbeat: Default {},
        1 => Identify { #[serde(flatten)] inner: Identify },
        2 => Resume {
            session: Snowflake,
        },
        3 => SetPresence { #[serde(flatten)] inner: SetPresence }
    }
}
