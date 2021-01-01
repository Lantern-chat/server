use serde_repr::{Deserialize_repr, Serialize_repr};

use serde::de::{self, Deserialize, Deserializer, MapAccess, SeqAccess, Visitor};

// Utility function where the
fn is_default<T>(value: &T) -> bool
where
    T: Default + Eq,
{
    *value == T::default()
}

macro_rules! decl_msgs {
    ($($code:expr => $opcode:ident $(:$Default:ident)? { $( $(#[$field_meta:meta])* $field:ident : $ty:ty),* }),*$(,)*) => {paste::paste!{
        #[derive(Debug, Clone, Copy, Serialize_repr, Deserialize_repr)]
        #[repr(u8)]
        pub enum Opcode {
            $($opcode = $code,)*
        }

        pub mod payloads {$(
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
                            _ => Err(de::Error::custom("Invalid opcode")),
                        }
                    }
                }

                deserializer.deserialize_struct("Message", &["o", "p"], MessageVisitor)
            }
        }
    }}
}

decl_msgs! {
    0 => Hello {
        /// Number of milliseconds between heartbeats
        heartbeat_interval: u32
    },
    1 => Heartbeat: Default {},
    2 => HeartbeatACK: Default {},
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hb() {
        let hb = r#"{"o": 2}"#;

        let p: Message = serde_json::from_str(hb).unwrap();

        let x = serde_json::to_string(&p).unwrap();

        println!("{:?}", p);
        println!("{}", x);
    }
}
