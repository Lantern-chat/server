use ftl::*;

use crate::{
    backend::api::oembed::get::{process_oembed, OEmbedFormat, OEmbedRequest, OEmbedResponse},
    Error, ServerState,
};

#[async_recursion]
pub async fn oembed(route: Route<ServerState>) -> Result<Response, Error> {
    let req = match route.query::<OEmbedRequest>() {
        Some(res) => res?,
        None => OEmbedRequest::default(),
    };

    let resp = process_oembed(&route.state, &req).await?;

    let resp = match req.format {
        OEmbedFormat::Json => serde_json::to_string(&resp)?,
        OEmbedFormat::XML => {
            let mut writer = Vec::from(r#"<?xml version="1.0" encoding="utf-8" standalone="yes"?><oembed>"#);

            quick_xml::se::to_writer(&mut writer, &resp)?;

            writer.extend_from_slice(b"</oembed>");

            unsafe { String::from_utf8_unchecked(writer) }
        }
    };

    Ok(resp.into_response())
}
