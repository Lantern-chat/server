use reqwest::StatusCode;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Invalid URL")]
    InvalidUrl,

    #[error("Failure: {0}")]
    Failure(StatusCode),

    #[error("Invalid MIME Type")]
    InvalidMimeType,

    #[error("JSON Error: {0}")]
    JsonError(#[from] serde_json::Error),

    #[error("XML Error: {0}")]
    XMLError(#[from] quick_xml::de::DeError),

    #[error(transparent)]
    ReqwestError(#[from] reqwest::Error),

    #[error(transparent)]
    UrlError(#[from] url::ParseError),
}
