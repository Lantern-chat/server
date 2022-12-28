use super::*;

#[async_recursion]
pub async fn attachments(mut route: Route<ServerState>) -> WebResult {
    let Some(Ok(room_id)) = route.next().param::<Snowflake>() else { return Err(Error::BadRequest) };
    let Some(Ok(attachment_id)) = route.next().param::<Snowflake>() else { return Err(Error::BadRequest) };

    let filename: smol_str::SmolStr = match route.next().segment() {
        Exact(filename) => urlencoding::decode(filename)?.into(),
        _ => return Err(Error::BadRequest),
    };

    let is_head = route.method() == Method::HEAD;

    let download = route.raw_query() == Some("download");

    crate::backend::cdn::get_attachment(route, room_id, attachment_id, Some(&filename), is_head, download)
        .await
        .map(From::from)
}
