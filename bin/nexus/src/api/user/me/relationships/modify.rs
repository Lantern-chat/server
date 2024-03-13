use futures::{Stream, StreamExt};
use schema::SnowflakeExt;

use crate::{prelude::*, util::encrypted_asset::encrypt_snowflake_opt};

use sdk::models::*;

pub async fn modify_relationship(
    state: ServerState,
    auth: Authorization,
    user_id: Snowflake,
    form: sdk::api::commands::user::PatchRelationshipBody,
) -> Result<(), Error> {
    if form.note.is_undefined() && form.rel.is_undefined() {
        return Err(Error::BadRequest);
    }

    Ok(())
}
