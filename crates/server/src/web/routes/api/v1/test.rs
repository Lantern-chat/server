use ftl::*;

use schema::{Snowflake, SnowflakeExt};
use smol_str::SmolStr;

use crate::ServerState;

pub async fn test(route: Route<ServerState>) -> Response {
    let state = route.state;

    use schema::*;
    use thorn::*;

    let ids = vec![
        Snowflake::from_i64(259579731467304960),
        Snowflake::from_i64(267769887873564672),
    ];

    let res = state
        .read_db()
        .await
        .query_cached_typed(
            || {
                Query::select()
                    .col(Users::Username)
                    .from_table::<Users>()
                    .and_where(Users::Id.equals(Builtin::any(Var::of(SNOWFLAKE_ARRAY))))
            },
            &[&ids],
        )
        .await
        .unwrap();

    for row in res {
        println!("{}", row.get::<_, SmolStr>(0));
    }

    ().into_response()
}
