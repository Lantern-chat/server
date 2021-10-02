use std::net::{IpAddr, SocketAddr};
use std::sync::Arc;

use arc_swap::ArcSwap;
use iplist::IpSet;

pub trait AddrFilter {
    #[inline]
    fn allow(&self, _addr: &IpAddr) -> bool {
        true
    }
}

impl AddrFilter for () {}

pub struct IpFilter {
    pub filter: ArcSwap<IpSet>,
}

impl Default for IpFilter {
    fn default() -> Self {
        IpFilter {
            filter: ArcSwap::from_pointee(IpSet::new(&*super::hardcoded_ip_bans::HARDCODED_IP_BANS)),
        }
    }
}

impl AddrFilter for IpFilter {
    fn allow(&self, addr: &IpAddr) -> bool {
        !self.filter.load().contains(&addr)
    }
}

impl IpFilter {
    pub fn store(&self, set: IpSet) {
        self.filter.store(Arc::new(set));
    }

    pub fn add(&self, ip: IpAddr) {
        self.filter.rcu(|set| {
            let mut set = IpSet::clone(&set);
            set.add(ip);
            set
        });
    }

    pub fn refresh(&self, ips: &[IpAddr]) {
        self.filter.rcu(|set| {
            let mut set = IpSet::clone(&set);
            set.refresh(ips);
            set
        });
    }
}
