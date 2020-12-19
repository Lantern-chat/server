use tokio_postgres::{
    types::{ToSql, Type},
    Client, Error, Row, RowStream, Statement,
};

pub struct PreparedQueryCache {
    queries: Vec<Statement>,
}

impl PreparedQueryCache {
    pub fn get(&self, query: CachedQuery) -> &Statement {
        unsafe { self.queries.get_unchecked((query as u16 - 1) as usize) }
    }
}

macro_rules! decl_queries {
    (@INTERNAL $client:expr, $($query:expr),+ => $tys:expr)  => { $client.prepare_typed(&format!($($query),+), $tys) };
    (@INTERNAL $client:expr, $($query:expr),+)               => { $client.prepare(&format!($($query),+)) };
    ($($(#[$meta:meta])* $name:ident => $($query:expr),+ $(=> $tys:expr)?;)*) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
        #[repr(u16)]
        pub enum CachedQuery { __FIRST = 0, $($(#[$meta])* $name),* }
        impl PreparedQueryCache {
            pub async fn populate(client: &Client) -> Result<PreparedQueryCache, Error> {
                let mut queries = Vec::default();
                // TODO: Replace with `futures::join!` for parallel preparation (faster startup)
                $(queries.push(decl_queries!(@INTERNAL client, $($query),+ $(=> $tys)?).await?);)*
                Ok(PreparedQueryCache { queries })
            }
        }
    }
}

decl_queries! {
    /// Selects all roles associated with a Party
    GetPartyRoles => "SELECT * FROM role WHERE party_id = $1"   => &[Type::INT8];

    /// Get owner User for Party
    GetPartyOwner => "SELECT * FROM user WHERE id = $1"         => &[Type::INT8];

    /// Get many attachments for message
    GetMessageAttachments => "SELECT * FROM attachment WHERE message_id = $1" => &[Type::INT8];

    /// Get thread this message is within
    GetMessageThread => "SELECT * FROM thread WHERE id = $1" => &[Type::INT8];
}
