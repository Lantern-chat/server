use super::*;

//pub mod check;
pub mod get;
pub mod register;

pub mod me;

pub fn user(mut route: Route<ServerState>, auth: MaybeAuth) -> RouteResult {
    match route.next().method_segment() {
        // POST /api/v1/user
        (&Method::POST, End) => Ok(register::register(route)),

        // ANY /api/v1/user/@me
        (_, Exact("@me")) => me::me(route, auth),

        // ANY /api/v1/user/1234
        (_, Exact(segment)) => {
            let Ok(user_id) = segment.parse::<Snowflake>() else {
                return err(CommonError::BadRequest);
            };

            let auth = auth.unwrap()?;

            match route.next().method_segment() {
                // GET /api/v1/user/1234
                (&Method::GET, End) => Ok(get::get(route, auth, user_id)),
                _ => err(CommonError::Unimplemented),
            }
        }
        _ => err(CommonError::NotFound),
    }
}
