use rpc::auth::Authorization;
use sdk::Snowflake;

#[inline(never)]
#[no_mangle]
fn get_user_id(auth: &Authorization) -> Snowflake {
    auth.user_id()
}

fn main() {}
