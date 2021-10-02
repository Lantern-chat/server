#![allow(unused)]

use std::{
    cmp::Ordering,
    net::{IpAddr, Ipv4Addr, Ipv6Addr},
    ops::Range,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
enum Kind {
    Ipv4 = 0,
    Ipv6 = 1,
    Ipv4Range = 2,
    Ipv6Range = 3,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(transparent)]
struct IpIndex(u64);

struct IpValues {
    ipv4: Vec<Ipv4Addr>,
    ipv6: Vec<Ipv6Addr>,
    ipv4r: Vec<Range<Ipv4Addr>>,
    ipv6r: Vec<Range<Ipv6Addr>>,
}

pub struct IpList {
    values: IpValues,
    sorted: Vec<IpIndex>,
}

impl IpIndex {
    fn decode(self) -> (Kind, usize) {
        let kind = (self.0 >> 62) as u8;
        let index = self.0 & (0b11 << 62);

        (unsafe { std::mem::transmute(kind) }, index as usize)
    }

    fn encode(kind: Kind, index: usize) -> IpIndex {
        debug_assert!(index <= (u64::MAX >> 2) as usize);

        IpIndex(((kind as u64) << 62) | index as u64)
    }
}

fn cmp_range_item<T>(a: T, range: &Range<T>) -> Ordering
where
    T: PartialOrd,
{
    if a < range.start {
        Ordering::Less
    } else if a > range.end {
        Ordering::Greater
    } else {
        Ordering::Equal
    }
}

fn cmp_range_full<T>(a: &Range<T>, b: &Range<T>) -> Ordering
where
    T: PartialOrd,
{
    if a.start < b.start {
        Ordering::Less
    } else if a.end > b.end {
        Ordering::Greater
    } else {
        Ordering::Equal
    }
}

impl IpValues {
    fn compare(&self, a: IpIndex, b: IpIndex) -> Ordering {
        let IpValues {
            ipv4,
            ipv6,
            ipv4r,
            ipv6r,
        } = self;

        let (ak, ai) = a.decode();
        let (bk, bi) = b.decode();

        match (ak, bk) {
            // 4 < 6
            (Kind::Ipv4 | Kind::Ipv4Range, Kind::Ipv6 | Kind::Ipv6Range) => Ordering::Less,
            // 6 > 4
            (Kind::Ipv6 | Kind::Ipv6Range, Kind::Ipv4 | Kind::Ipv4Range) => Ordering::Greater,

            (Kind::Ipv4, Kind::Ipv4) => ipv4[ai].cmp(&ipv4[bi]),
            (Kind::Ipv6, Kind::Ipv6) => ipv6[ai].cmp(&ipv6[bi]),

            (Kind::Ipv4, Kind::Ipv4Range) => cmp_range_item(ipv4[ai], &ipv4r[bi]),
            (Kind::Ipv4Range, Kind::Ipv4) => cmp_range_item(ipv4[bi], &ipv4r[ai]).reverse(),
            (Kind::Ipv6, Kind::Ipv6Range) => cmp_range_item(ipv6[ai], &ipv6r[bi]),
            (Kind::Ipv6Range, Kind::Ipv6) => cmp_range_item(ipv6[bi], &ipv6r[ai]).reverse(),

            (Kind::Ipv4Range, Kind::Ipv4Range) => cmp_range_full(&ipv4r[ai], &ipv4r[bi]),
            (Kind::Ipv6Range, Kind::Ipv6Range) => cmp_range_full(&ipv6r[ai], &ipv6r[bi]),
        }
    }

    fn compare_ip(&self, idx: IpIndex, ip: IpAddr) -> Ordering {
        let (k, idx) = idx.decode();

        match (k, ip) {
            (Kind::Ipv4 | Kind::Ipv4Range, IpAddr::V6(_)) => Ordering::Less,
            (Kind::Ipv6 | Kind::Ipv6Range, IpAddr::V4(_)) => Ordering::Greater,

            (Kind::Ipv4, IpAddr::V4(ip)) => self.ipv4[idx].cmp(&ip),
            (Kind::Ipv6, IpAddr::V6(ip)) => self.ipv6[idx].cmp(&ip),

            (Kind::Ipv4Range, IpAddr::V4(ip)) => cmp_range_item(ip, &self.ipv4r[idx]),
            (Kind::Ipv6Range, IpAddr::V6(ip)) => cmp_range_item(ip, &self.ipv6r[idx]),
        }
    }

    fn insert(&mut self, ip: IpAddr) -> IpIndex {
        match ip {
            IpAddr::V4(ip) => {
                let idx = self.ipv4.len();
                self.ipv4.push(ip);
                IpIndex::encode(Kind::Ipv4, idx)
            }
            IpAddr::V6(ip) => {
                let idx = self.ipv6.len();
                self.ipv6.push(ip);
                IpIndex::encode(Kind::Ipv6, idx)
            }
        }
    }
}

impl IpList {
    pub fn sort(&mut self) {
        let IpList {
            ref values,
            ref mut sorted,
        } = *self;

        sorted.sort_unstable_by(|a, b| values.compare(*a, *b))
    }

    pub fn contains(&self, ip: IpAddr) -> bool {
        self.sorted
            .binary_search_by(|idx| self.values.compare_ip(*idx, ip))
            .is_ok()
    }

    pub fn insert(&mut self, ip: IpAddr) {
        if let Err(idx) = self
            .sorted
            .binary_search_by(|idx| self.values.compare_ip(*idx, ip))
        {
            self.sorted.insert(idx, self.values.insert(ip));
        }
    }
}

use hashbrown::{hash_map::DefaultHashBuilder, raw::RawTable};

#[derive(Default, Clone)]
pub struct IpSet {
    ipv4: Vec<Ipv4Addr>,
    ipv6: Vec<Ipv6Addr>,
    set: RawTable<u32>,
    hash_builder: DefaultHashBuilder,
}

const MAX_LEN: usize = 1 << 31;
const INDEX_MASK: usize = 0x7FFFFFFF;

impl IpSet {
    pub fn new(ips: &[IpAddr]) -> Self {
        let mut this = IpSet {
            ipv4: Vec::new(),
            ipv6: Vec::new(),
            set: RawTable::new(),
            hash_builder: DefaultHashBuilder::new(),
        };

        this.refresh(ips);

        this
    }

    // Recreate IpSet without freeing memory
    pub fn refresh(&mut self, ips: &[IpAddr]) {
        use std::hash::{BuildHasher, Hash, Hasher};

        self.ipv4.clear();
        self.ipv6.clear();

        self.set.clear();
        self.set.reserve(ips.len(), |_| 0); // there are no elements to rehash

        for &ip in ips {
            let mut hasher = self.hash_builder.build_hasher();

            let idx = match ip {
                IpAddr::V4(ip) => {
                    ip.hash(&mut hasher);
                    let idx = self.ipv4.len();
                    assert!(idx < MAX_LEN);
                    self.ipv4.push(ip);
                    idx as u32
                }
                IpAddr::V6(ip) => {
                    ip.hash(&mut hasher);
                    let idx = self.ipv6.len();
                    assert!(idx < MAX_LEN);
                    self.ipv6.push(ip);
                    idx as u32 | (1 << 31)
                }
            };

            self.set.insert_no_grow(hasher.finish(), idx);
        }
    }

    fn hash(&self, ip: &IpAddr) -> u64 {
        use std::hash::{BuildHasher, Hash, Hasher};

        let mut hasher = self.hash_builder.build_hasher();

        // avoid hashing the discriminant
        match ip {
            IpAddr::V4(ip) => ip.hash(&mut hasher),
            IpAddr::V6(ip) => ip.hash(&mut hasher),
        }

        hasher.finish()
    }

    #[inline]
    unsafe fn cmp_eq(&self, idx: u32, ip: &IpAddr) -> bool {
        match (idx >> 31 == 0, ip) {
            (true, IpAddr::V4(ip)) => ip.eq(self.ipv4.get_unchecked(idx as usize)),
            (false, IpAddr::V6(ip)) => ip.eq(self.ipv6.get_unchecked(idx as usize & INDEX_MASK)),
            _ => false,
        }
    }

    //unsafe fn get(&self, idx: u32) -> IpAddr {
    //    if idx >> 31 == 0 {
    //        IpAddr::V4(*self.ipv4.get_unchecked(idx as usize))
    //    } else {
    //        IpAddr::V6(*self.ipv6.get_unchecked(idx as usize & INDEX_MASK))
    //    }
    //}

    pub fn contains(&self, ip: &IpAddr) -> bool {
        self.set
            .find(self.hash(ip), |idx| unsafe { self.cmp_eq(*idx, ip) })
            .is_some()
    }

    pub fn add(&mut self, ip: IpAddr) {
        let hash = self.hash(&ip);

        let idx = match ip {
            IpAddr::V4(ip) => {
                let idx = self.ipv4.len();
                assert!(idx < MAX_LEN);
                self.ipv4.push(ip);
                idx as u32
            }
            IpAddr::V6(ip) => {
                let idx = self.ipv6.len();
                assert!(idx < MAX_LEN);
                self.ipv6.push(ip);
                idx as u32 | (1 << 31)
            }
        };

        let IpSet {
            ref hash_builder,
            ref mut set,
            ref ipv4,
            ref ipv6,
        } = self;

        set.insert(hash, idx, |&idx| unsafe {
            use std::hash::{BuildHasher, Hash, Hasher};

            let mut hasher = hash_builder.build_hasher();

            if idx >> 31 == 0 {
                ipv4.get_unchecked(idx as usize).hash(&mut hasher);
            } else {
                ipv6.get_unchecked(idx as usize & INDEX_MASK).hash(&mut hasher);
            }

            hasher.finish()
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ipset() {
        let banned = vec![
            "2604:f4a0:2:a::b75e".parse().unwrap(),
            "65.105.159.243".parse().unwrap(),
        ];

        let set = IpSet::new(&banned);

        for banned in &banned {
            assert!(set.contains(banned));
        }
    }
}
