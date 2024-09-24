pub mod api_v1 {}

use crate::prelude::*;
use ftl::Router;

pub fn router(state: ServerState) -> Router<ServerState> {
    use sdk::api::{commands::all as cmds, Command};

    let mut router = Router::with_state(state);

    router.get("/robots.txt", robots);

    macro_rules! add_routes {
        ($($cmd:ty: $handler:expr),+ $(,)?) => {
            router $(.on(
                &[<$cmd as Command>::HTTP_METHOD],
                <$cmd as Command>::ROUTE_PATTERN,
                $handler,
            ))+;
        };
    }

    add_routes! {
        cmds::GetBuildInfo: api_v1::build::build_info,
    };

    router
}
