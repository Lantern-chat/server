use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};
use std::sync::LazyLock;

pub static HARDCODED_IP_BANS: LazyLock<Vec<IpAddr>> = LazyLock::new(|| {
    vec![
        //"2607:f7a0:1:d::b75e".parse().unwrap(),
        //"66.115.189.233".parse().unwrap(),
    ]
});
