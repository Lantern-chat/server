use ftl::*;

use crate::{
    ctrl::{
        oembed::get::{process_oembed, OEmbedFormat, OEmbedRequest, OEmbedResponse},
        Error,
    },
    web::routes::api::ApiError,
    ServerState,
};

pub async fn oembed(route: Route<ServerState>) -> Response {
    let req = match route.query::<OEmbedRequest>() {
        Some(Ok(req)) => req,
        None => OEmbedRequest::default(),
        Some(Err(e)) => return ApiError::err(e.into()).into_response(),
    };

    let resp = match process_oembed(route.state, &req).await {
        Ok(resp) => resp,
        Err(e) => return ApiError::err(e).into_response(),
    };

    let resp = match req.format {
        OEmbedFormat::Json => serde_json::to_string(&resp).map_err(Error::from),
        OEmbedFormat::XML => {
            let mut writer = Vec::from(r#"<?xml version="1.0" encoding="utf-8" standalone="yes"?><oembed>"#);

            match quick_xml::se::to_writer(&mut writer, &resp) {
                Ok(_) => {
                    writer.extend_from_slice(b"</oembed>");

                    Ok(unsafe { String::from_utf8_unchecked(writer) })
                }
                Err(e) => Err(Error::from(e)),
            }
        }
    };

    resp.into_response()
}
