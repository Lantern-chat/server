use super::*;

use sdk::api::commands::all as cmds;

pub async fn register(cmd: cmds::UserRegister) -> Procedure {
    Procedure::from(cmd)
}
