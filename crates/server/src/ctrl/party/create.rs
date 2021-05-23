use db::SnowflakeExt;

use models::*;

use crate::{
    ctrl::{auth::Authorization, Error},
    ServerState,
};

#[derive(Debug, Clone, Deserialize)]
pub struct PartyCreateForm {
    name: String,
    //description: String,
    //security: SecurityFlags,
}

pub async fn create_party(
    state: ServerState,
    auth: Authorization,
    form: PartyCreateForm,
) -> Result<Party, Error> {
    if !state.config.partyname_len.contains(&form.name.len()) {
        return Err(Error::InvalidName);
    }

    let party = Party {
        partial: PartialParty {
            id: Snowflake::now(),
            name: form.name,
            description: None,
        },
        owner: auth.user_id,
        security: SecurityFlags::empty(),
        roles: Vec::new(),
        emotes: Vec::new(),
    };

    Ok(party)
}
