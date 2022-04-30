use std::{net::SocketAddr, str::FromStr};

use super::util;

section! {
    #[derive(Debug, Serialize, Deserialize)]
    #[serde(default)]
    pub struct General {
        pub server_name: String = "Lantern Chat".to_owned() => "LANTERN_SERVER_NAME",
        pub bind: SocketAddr = SocketAddr::from(([127, 0, 0, 1], 8080)) => "LANTERN_BIND" | parse_address,
        pub cdn_domain: String = "cdn.lanternchat.net".to_owned() => "LANTERN_CDN_DOMAIN",
        pub base_domain: String = "lantern.chat".to_owned() => "LANTERN_BASE_DOMAIN",
        pub https: bool = true => "LANTERN_HTTPS" | util::parse[true],
        pub instance_id: u16 = 0 => "LANTERN_INSTANCE_ID" | util::parse[0u16],
        pub worker_id: u16 = 0 => "LANTERN_WORKER_ID" | util::parse[0u16],
    }
}

fn parse_address(value: &str) -> SocketAddr {
    SocketAddr::from_str(&value.replace("localhost", "127.0.0.1")).unwrap()
}

impl General {
    pub fn configure(&self) {
        use schema::sf;

        unsafe {
            sf::INST = self.instance_id;
            sf::WORK = self.worker_id;
        }
    }

    pub fn base_url(&self) -> String {
        format!("http{}://{}", if self.https { "s" } else { "" }, self.base_domain)
    }
}
