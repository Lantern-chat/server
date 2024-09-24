use ftl::{
    body::deferred::Deferred,
    service::{Service, ServiceFuture},
    IntoResponse, Request, Response, Router,
};

use crate::prelude::*;

pub mod build;
pub mod api {
    pub mod v1;
}

pub struct WebService {
    pub web: Router<ServerState, Response>,
    pub api_v1: api::v1::ApiV1Service,
}

impl Service<Request> for WebService {
    type Error = Error;
    type Response = Response;

    fn call(&self, req: Request) -> impl ServiceFuture<Self::Response, Self::Error> {
        async move {
            if req.uri().path().starts_with("/api/v1/") {
                return self.api_v1.call(req).await;
            }

            match self.web.call(req).await {
                Ok(resp) => Ok(resp),
                Err(e) => Ok(e.into_response()),
            }
        }
    }
}

impl WebService {
    pub fn new(state: ServerState) -> Self {
        let mut web = Router::with_state(state.clone());

        web.get("/robots.txt", robots);
        web.get("/build", build::build_info);

        Self {
            web,
            api_v1: api::v1::ApiV1Service::new(state.clone()),
        }
    }
}

pub async fn robots() -> &'static str {
    include_str!("robots.txt")
}
