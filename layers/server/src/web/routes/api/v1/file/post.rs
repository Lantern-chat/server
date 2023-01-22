use super::*;

#[async_recursion]
pub async fn post(mut route: Route<ServerState>, auth: Authorization) -> WebResult {
    let body = body::any(&mut route).await?;

    let file_id = crate::backend::api::file::post::post_file(&route.state, auth.user_id, body).await?;

    let mut res = crate::web::response::wrap_response_once(&route, |_| {
        Ok(WebResponse::new(file_id).with_status(StatusCode::CREATED))
    });

    res.headers_mut().extend(super::tus_headers());

    res.headers_mut().insert(
        HeaderName::from_static("Location"),
        super::header_from_int(file_id.to_u64()),
    );

    Ok(res.into())
}
