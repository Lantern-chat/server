use super::*;

//use crate::oembed::get::{process_oembed, OEmbedFormat, OEmbedRequest, OEmbedResponse};

#[async_recursion]
pub async fn oembed(route: Route<ServerState>) -> ApiResult {
    Err(Error::Unimplemented)

    // let req = match route.query::<OEmbedRequest>() {
    //     Some(res) => res?,
    //     None => OEmbedRequest::default(),
    // };

    // let resp = process_oembed(&route.state, &req).await?;

    // let resp = match req.format {
    //     OEmbedFormat::Json => serde_json::to_string(&resp)?,
    //     OEmbedFormat::XML => {
    //         let mut writer = r#"<?xml version="1.0" encoding="utf-8" standalone="yes"?><oembed>"#.to_owned();

    //         quick_xml::se::to_writer(&mut writer, &resp)?;

    //         writer.push_str("</oembed>");

    //         writer
    //     }
    // };

    // Ok(resp.into())
}
