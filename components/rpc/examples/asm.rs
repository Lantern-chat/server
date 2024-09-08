use rkyv::option::ArchivedOption;
use rpc::{auth::Authorization, DeserializeExt};
use sdk::models::*;

fn main() {}

#[inline(never)]
#[no_mangle]
pub fn get_user_id(auth: &Authorization) -> Snowflake {
    auth.user_id()
}

#[inline(never)]
#[no_mangle]
pub fn simple_deserialize_cursor(cursor: &ArchivedOption<ArchivedCursor>) -> Cursor {
    match cursor.deserialize_simple().expect("Unable to deserialize cursor") {
        Some(query) => query,
        _ => Cursor::Before(MessageId::max_safe_value()),
    }
}
