use super::*;

pub mod get;
pub mod profile;

pub fn members(mut route: Route<ServerState>, auth: Authorization, party_id: Snowflake) -> RouteResult {
    match route.next().method_segment() {
        // GET /api/v1/party/1234/members
        (&Method::GET, End) => Ok(get::get_members(route, auth, party_id)),

        // GET /api/v1/party/1234/members/5678
        (&Method::GET, Exact(segment)) => {
            let Ok(member_id) = segment.parse::<Snowflake>() else {
                return Err(Error::BadRequest);
            };

            match route.next().segment() {
                End => Ok(get::get_member(route, auth, party_id, member_id)),
                _ => Err(Error::NotFound),
            }
        }
        _ => Err(Error::NotFound),
    }
}
