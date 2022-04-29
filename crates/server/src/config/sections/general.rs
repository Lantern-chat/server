use std::{net::SocketAddr, str::FromStr};

section! {
    #[derive(Debug, Serialize, Deserialize)]
    #[serde(default)]
    pub struct General {
        pub server_name: String = "Lantern Chat".to_owned(),
        pub bind: SocketAddr = SocketAddr::from(([127, 0, 0, 1], 8080)) => "LANTERN_BIND" | parse_address,
        pub cdn_domain: String = "cdn.lanternchat.net".to_owned() => "LANTERN_CDN",
    }
}

fn parse_address(value: &str) -> SocketAddr {
    SocketAddr::from_str(&value.replace("localhost", "127.0.0.1")).unwrap()
}
