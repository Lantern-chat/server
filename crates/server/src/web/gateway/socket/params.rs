use crate::web::encoding::Encoding;

const fn default_compress() -> bool {
    true
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct GatewayQueryParams {
    /// Encoding method for each individual websocket message
    #[serde(default)]
    pub encoding: Encoding,

    /// Whether to compress individual messages
    #[serde(default = "default_compress")]
    pub compress: bool,
}

impl Default for GatewayQueryParams {
    fn default() -> Self {
        GatewayQueryParams {
            encoding: Encoding::default(),
            compress: default_compress(),
        }
    }
}
