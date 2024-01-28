use super::*;

pub async fn options(route: Route<ServerState>, auth: Authorization) -> ApiResult {
    let options = crate::backend::api::file::options::file_options(&route.state, auth).await?;

    // want the body formatted based on query, but we need the response back to fill out headers...
    let mut res = crate::web::response::wrap_response_once(&route, |_| Ok(WebResponse::new(options)));

    let headers = res.headers_mut();

    headers.extend(super::tus_headers());

    headers.insert(
        HeaderName::from_static("upload-quota-used"),
        super::header_from_int(options.quota_used),
    );
    headers.insert(
        HeaderName::from_static("upload-quota-total"),
        super::header_from_int(options.quota_total),
    );

    Ok(res.into())
}
