use crate::{api::auth, Error, State};

use sdk::models::UserPreferences;
use thorn::pg::Json;

pub async fn update_prefs(
    state: State,
    auth: auth::Authorization,
    mut prefs: UserPreferences,
) -> Result<(), Error> {
    if let Err(e) = prefs.validate() {
        return Err(Error::InvalidPreferences(e));
    }

    prefs.nullify_defaults();

    let db = state.db.write.get().await?;

    db.execute_cached_typed(
        || {
            use schema::*;
            use thorn::*;

            Query::update()
                .table::<Users>()
                .set(
                    Users::Preferences,
                    // defaults are set to null, so strip them to save space
                    Call::custom("jsonb_strip_nulls").arg(
                        // Coalesce in case user never had prefs
                        Builtin::coalesce((Users::Preferences, Literal::EMPTY_ARRAY.cast(Type::JSONB)))
                            // concat to overwrite old prefs
                            .concat(Var::of(Users::Preferences)),
                    ),
                )
                .and_where(Users::Id.equals(Var::of(Users::Id)))
        },
        &[&Json(prefs), &auth.user_id],
    )
    .await?;

    Ok(())
}
