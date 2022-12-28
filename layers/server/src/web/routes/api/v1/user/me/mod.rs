use super::*;

pub mod account;
pub mod billing;
//pub mod friends;
pub mod login;
pub mod logout;
pub mod prefs;
pub mod profile;
pub mod sessions;

pub fn me(mut route: Route<ServerState>, auth: MaybeAuth) -> RouteResult {
    match route.next().method_segment() {
        // POST /api/v1/user/@me
        (&Method::POST, End) => Ok(login::login(route)),

        // Everything else requires authorization
        _ => {
            let auth = auth.unwrap()?;

            match route.method_segment() {
                (&Method::DELETE, End) => Ok(logout::logout(route, auth)),
                (&Method::GET, Exact("sessions")) => Ok(sessions::sessions(route, auth)),
                (&Method::PATCH, Exact("prefs")) => Ok(prefs::prefs(route, auth)),
                (&Method::PATCH, Exact("account")) => Ok(account::patch_account(route, auth)),
                (&Method::PATCH, Exact("profile")) => Ok(profile::patch_profile(route, auth)),
                // (_, Exact("friends")) => {
                //     // bots cannot have friends :(
                //     if auth.is_bot() {
                //         return Err(Error::BadRequest);
                //     }

                //     match route.next().method_segment() {
                //         (&Method::GET, End) => friends::get(route, auth).await,
                //         (_, Exact(_)) => {
                //             let Some(Ok(user_id)) = route.param::<Snowflake>() else {
                //                 return Err(Error::BadRequest);
                //             };

                //             match route.method() {
                //                 &Method::POST => friends::post(route, auth, user_id).await,
                //                 &Method::DELETE => friends::del(route, auth, user_id).await,
                //                 &Method::PATCH => todo!("PatchFriend"),
                //                 _ => Err(Error::MethodNotAllowed)
                //             }
                //         }
                //         _ => Err(Error::NotFound),
                //     }
                // },
                _ => Err(Error::NotFound),
            }
        }
    }
}
