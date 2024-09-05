use crate::prelude::*;
use sdk::{api::commands::all::UpdateUserPrefs, models::UserPreferences};
use thorn::pg::Json;

pub async fn update_prefs(
    state: ServerState,
    auth: Authorization,
    cmd: &Archived<UpdateUserPrefs>,
) -> Result<(), Error> {
    let mut prefs: UserPreferences = cmd.body.inner.deserialize_full().expect("Unable to deserialize prefs");

    prefs.clean();

    let db = state.db.write.get().await?;
    let prefs = Json(prefs);

    db.execute2(schema::sql! {
        UPDATE Users SET (Preferences) = (
            // defaults are set to null, so strip them to save space
            jsonb_strip_nulls(
                // Coalesce in case user never had prefs, then concat to overwrite old prefs
                COALESCE(Users.Preferences, "{}"::jsonb) || #{&prefs as Users::Preferences}
            )
        ) WHERE Users.Id = #{auth.user_id_ref() as Users::Id}
    })
    .await?;

    Ok(())
}
