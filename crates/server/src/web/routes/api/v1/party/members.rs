use ftl::*;
use schema::Snowflake;

use crate::{ctrl::auth::Authorization, ctrl::Error, web::routes::api::ApiError, ServerState};

pub async fn get_members(route: Route<ServerState>, auth: Authorization, party_id: Snowflake) -> Response {
    let is_member = route
        .state
        .read_db()
        .await
        .query_opt_cached_typed(
            || {
                use schema::*;
                use thorn::*;

                Query::select()
                    .from_table::<PartyMember>()
                    .and_where(PartyMember::PartyId.equals(Var::of(Party::Id)))
                    .and_where(PartyMember::UserId.equals(Var::of(Users::Id)))
            },
            &[&party_id, &auth.user_id],
        )
        .await;

    match is_member {
        Err(e) => ApiError::err(e.into()).into_response(),
        Ok(None) => ApiError::err(Error::NoSession).into_response(),
        Ok(Some(_)) => match crate::ctrl::party::members::get_members(route.state, party_id).await {
            Ok(stream) => reply::json_stream(stream).into_response(),
            Err(e) => ApiError::err(e).into_response(),
        },
    }
}
