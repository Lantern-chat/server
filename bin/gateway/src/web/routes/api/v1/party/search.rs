use http::{HeaderName, HeaderValue};

use super::*;

#[async_recursion]
pub async fn search(mut route: Route<ServerState>, auth: Authorization, party_id: Snowflake) -> WebResult {
    match route.content_length() {
        None => return Err(Error::BadRequest),
        Some(len) if len > (1024 * 1024) => return Err(Error::RequestEntityTooLarge),
        _ => {}
    }

    let body = route.bytes().await.map_err(body::BodyDeserializeError::from)?;

    let terms = schema::search::parse_search_terms(std::str::from_utf8(&body)?)?;

    unimplemented!()

    // let res = crate::backend::api::room::messages::get::get_search(route.state, auth, party_id, terms).await?;

    // let count = res.lower_bound.load(std::sync::atomic::Ordering::Relaxed);

    // Ok(WebResponse::stream(res.stream).with_header(
    //     HeaderName::from_static("count"),
    //     // save a few nanoseconds by not allocating for formatting in very common cases
    //     match count {
    //         // 1001 is the common limit when the full lower bound cannot be computed
    //         1001 => HeaderValue::from_static("1001"),
    //         0 => HeaderValue::from_static("0"),
    //         _ => HeaderValue::try_from(format!("{count}")).unwrap(),
    //     },
    // ))
}
