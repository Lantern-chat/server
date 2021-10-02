use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

lazy_static::lazy_static! {
    pub static ref HARDCODED_IP_BANS: Vec<IpAddr> = vec![
        //"2607:f7a0:1:d::b75e".parse().unwrap(),
        //"66.115.189.233".parse().unwrap(),

    ];
}
