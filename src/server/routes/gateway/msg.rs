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
    ($($opcode:ident $(:$Default:ident)? { $($field:ident : $ty:ty),* }),*) => {paste::paste!{
        #[derive(Debug, Clone, Copy, Serialize_repr, Deserialize_repr)]
        #[repr(u8)]
        pub enum Opcode {
            InvalidOpcode = 0,
            $($opcode,)*
        }

        pub mod payloads {$(
            #[derive(Debug, Clone, Serialize, Deserialize)]
            $(#[derive($Default, PartialEq, Eq)])?
            pub struct [<$opcode Payload>] {
                $($field : $ty,)*
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
                            _ => return Err(de::Error::custom("Missing opcode")),
                        };

                        match opcode {
                            $(
                                Opcode::$opcode => Ok(Message::$opcode {
                                    op: opcode,
                                    payload: match map.next_entry()? {
                                        Some((Field::Payload, payload)) => payload,
                                        $(None => $Default::default(),)?
                                        _ => return Err(de::Error::custom("Missing payload")),
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
    Hello { heartbeat_interval: u32 },
    Heartbeat: Default {},
    HeartbeatACK: Default {}
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
