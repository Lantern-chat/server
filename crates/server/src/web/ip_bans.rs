use std::convert::TryFrom;
use std::hash::{BuildHasher, Hash, Hasher};
use std::net::IpAddr;
use std::sync::Arc;

use ahash::AHasher;
use arc_swap::ArcSwap;
use futures::StreamExt;
use xorf::{Filter, Xor16};

use util::cmap::{CHashSet, DefaultHashBuilder};

use db::pool::Pool;

use crate::ctrl::Error;

struct IpBansInner {
    pub old: Xor16,
    pub new: CHashSet<IpAddr, DefaultHashBuilder>,
}

pub struct IpBans {
    inner: ArcSwap<IpBansInner>,
    hash_builder: DefaultHashBuilder,
}

impl IpBans {
    pub fn is_probably_banned(&self, ip: IpAddr) -> bool {
        let hash = {
            let mut hasher = self.hash_builder.build_hasher();
            ip.hash(&mut hasher);
            hasher.finish()
        };

        let inner = self.inner.load_full();

        // fast path with outdated condensed xor-filter
        if inner.old.contains(&hash) {
            return true;
        }

        inner.new.try_maybe_contains_hash(hash)
    }

    pub fn new() -> Self {
        let hash_builder = DefaultHashBuilder::new();
        IpBans {
            inner: ArcSwap::from_pointee(IpBansInner {
                old: Xor16::try_from(&[] as &'static [u64]).unwrap(),
                new: CHashSet::with_hasher(CHashSet::<()>::default_num_shards(), hash_builder.clone()),
            }),
            hash_builder,
        }
    }

    pub async fn refresh(&self, db: &Pool) -> Result<(), Error> {
        let conn = db.get().await?;

        let stream = conn
            .query_stream_cached_typed(
                || {
                    use schema::*;
                    use thorn::*;

                    Query::select().from_table::<IpBans>().cols(&[IpBans::Addr])
                },
                &[],
            )
            .await?;

        let mut hashes = Vec::new();

        futures::pin_mut!(stream);
        while let Some(row) = stream.next().await {
            let row = row?;
            let ip: IpAddr = row.try_get(0)?;

            let mut hasher = self.hash_builder.build_hasher();
            ip.hash(&mut hasher);
            hashes.push(hasher.finish());
        }

        let filter = Xor16::try_from(hashes).unwrap();

        self.inner.store(Arc::new(IpBansInner {
            old: filter,
            new: CHashSet::with_hasher(CHashSet::<()>::default_num_shards(), self.hash_builder.clone()),
        }));

        Ok(())
    }
}
