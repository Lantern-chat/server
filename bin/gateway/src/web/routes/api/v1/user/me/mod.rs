use super::*;

pub mod account;
pub mod billing;
pub mod mfa;
//pub mod friends;
pub mod login;
pub mod logout;
pub mod prefs;
pub mod profile;
pub mod relationships;
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
                (_, Exact("2fa")) => {
                    if auth.is_bot() {
                        // bots do not use 2fa
                        return Err(Error::BadRequest);
                    }

                    match *route.method() {
                        Method::POST => Ok(mfa::post_2fa(route, auth)),
                        Method::PATCH => Ok(mfa::patch_2fa(route, auth)),
                        Method::DELETE => Ok(mfa::delete_2fa(route, auth)),
                        _ => Err(Error::MethodNotAllowed),
                    }
                }
                (_, Exact("relationships")) => {
                    if auth.is_bot() {
                        // bots cannot have friends :(
                        return Err(Error::BadRequest);
                    }

                    match route.next().method_segment() {
                        (&Method::GET, End) => unimplemented!(),
                        (&Method::PATCH, Exact(_)) => match route.param::<Snowflake>() {
                            Some(Ok(user_id)) => {
                                unimplemented!();
                            }
                            _ => Err(Error::BadRequest),
                        },
                        _ => Err(Error::NotFoundSignaling),
                    }
                }
                _ => Err(Error::NotFoundSignaling),
            }
        }
    }
}
