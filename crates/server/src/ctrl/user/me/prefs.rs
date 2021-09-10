use crate::ctrl::{auth, Error};
use crate::ServerState;

use models::UserPreferences;
use thorn::pg::Json;

pub async fn update_prefs(
    state: ServerState,
    auth: auth::Authorization,
    prefs: UserPreferences,
) -> Result<(), Error> {
    if let Err(e) = prefs.validate() {
        return Err(Error::InvalidPreferences(e));
    }

    let db = state.db.write.get().await?;

    db.execute_cached_typed(
        || {
            use schema::*;
            use thorn::*;

            Query::update()
                .table::<Users>()
                .set(
                    Users::Preferences,
                    Builtin::coalesce((Users::Preferences, Literal::TextStr("{}").cast(Type::JSONB)))
                        .concat(Var::of(Users::Preferences)),
                )
                .and_where(Users::Id.equals(Var::of(Users::Id)))
        },
        &[&Json(prefs), &auth.user_id],
    )
    .await?;

    Ok(())
}
