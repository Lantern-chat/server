use std::{net::SocketAddr, str::FromStr};

use super::util;

section! {
    #[derive(Debug, Serialize, Deserialize)]
    #[serde(default)]
    pub struct Web {
        pub bind: SocketAddr = SocketAddr::from(([127, 0, 0, 1], 8080)) => "LANTERN_BIND" | parse_address,
        pub cdn_domain: String = "cdn.lanternchat.net".to_owned() => "LANTERN_CDN_DOMAIN",
        pub strict_cdn: bool = true,
        pub base_domain: String = "lantern.chat".to_owned() => "LANTERN_BASE_DOMAIN",
        pub https: bool = true => "LANTERN_HTTPS" | util::parse[true],
    }
}

fn parse_address(value: &str) -> SocketAddr {
    SocketAddr::from_str(&value.replace("localhost", "127.0.0.1")).unwrap()
}

impl Web {
    pub fn base_url(&self) -> String {
        format!("http{}://{}", if self.https { "s" } else { "" }, self.base_domain)
    }
}
