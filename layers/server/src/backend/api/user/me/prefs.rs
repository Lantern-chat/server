use crate::{Authorization, Error, ServerState};

use sdk::models::UserPreferences;
use thorn::pg::Json;

pub async fn update_prefs(
    state: ServerState,
    auth: Authorization,
    mut prefs: UserPreferences,
) -> Result<(), Error> {
    if let Err(e) = prefs.validate() {
        return Err(Error::InvalidUserPreferences(e));
    }

    prefs.nullify_defaults();

    let db = state.db.write.get().await?;
    let prefs = Json(prefs);

    db.execute2(schema::sql! {
        UPDATE Users SET (Preferences) = (
            // defaults are set to null, so strip them to save space
            jsonb_strip_nulls(
                // Coalesce in case user never had prefs, then concat to overwrite old prefs
                COALESCE(Users.Preferences, "{}"::jsonb) || #{&prefs => Users::Preferences}
            )
        ) WHERE Users.Id = #{&auth.user_id => Users::Id}
    }?)
    .await?;

    Ok(())
}
