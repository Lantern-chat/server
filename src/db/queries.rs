use hashbrown::HashMap;
use tokio_postgres::{
    types::{ToSql, Type},
    Client, Error, Row, RowStream, Statement,
};

pub struct PreparedQueryCache {
    queries: HashMap<CachedQuery, Statement>,
}

impl PreparedQueryCache {
    pub fn get(&self, query: CachedQuery) -> &Statement {
        self.queries
            .get(&query)
            .unwrap_or_else(|| unsafe { std::hint::unreachable_unchecked() })
    }
}

macro_rules! decl_queries {
    (@INTERNAL $client:expr, $query:expr => $tys:expr)  => { $client.prepare_typed($query, $tys) };
    (@INTERNAL $client:expr, $query:expr)               => { $client.prepare($query) };
    ($($name:ident => $query:expr $(=> $tys:expr)?;)*) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
        #[repr(u16)]
        pub enum CachedQuery { $($name),* }
        impl PreparedQueryCache {
            pub async fn populate(client: &Client) -> Result<PreparedQueryCache, Error> {
                let mut queries = HashMap::default();
                // TODO: Replace with `futures::join!` for parallel preparation (faster startup)
                $(queries.insert(
                    CachedQuery::$name,
                    decl_queries!(@INTERNAL client, $query $(=> $tys)?).await?
                );)*
                Ok(PreparedQueryCache { queries })
            }
        }
    }
}

decl_queries! {
    GetPartyRoles => "SELECT * FROM role WHERE party_id = $1"   => &[Type::INT8];
    GetPartyOwner => "SELECT * FROM user WHERE id = $1"         => &[Type::INT8];
}
