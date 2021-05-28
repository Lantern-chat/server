use enum_primitive::*;

enum_from_primitive! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub enum EventCode {
        MessageCreate = 1,
        MessageUpdate = 2,
        MessageDelete = 3,
    }
}
