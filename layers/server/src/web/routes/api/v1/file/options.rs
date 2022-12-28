use super::*;

#[async_recursion]
pub async fn options(route: Route<ServerState>, auth: Authorization) -> WebResult {
    let options = crate::backend::api::file::options::file_options(&route.state, auth).await?;

    // want the body formatted based on query, but we need the response back to fill out headers...
    let mut res = crate::web::response::wrap_response_once(&route, |_| Ok(WebResponse::new(options)));

    let headers = res.headers_mut();

    headers.extend(super::tus_headers());

    headers.insert("Upload-Quota-Used", super::header_from_int(options.quota_used));
    headers.insert("Upload-Quota-Total", super::header_from_int(options.quota_total));

    Ok(res.into())
}
