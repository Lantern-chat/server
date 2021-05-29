use enum_primitive::*;

enum_from_primitive! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub enum EventCode {
        MessageCreate = 1,
        MessageUpdate = 2,
        MessageDelete = 3,
    }
}

impl EventCode {
    pub fn from_i16(value: i16) -> Option<EventCode> {
        FromPrimitive::from_i16(value)
    }
}
