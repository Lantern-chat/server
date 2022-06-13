use futures::FutureExt;
use hyper::{
    server::conn::AddrStream,
    service::{make_service_fn, service_fn},
    Server,
};

use ftl::{Reply, StatusCode};

use super::*;

pub fn add_http_server_task(state: &ServerState, runner: &TaskRunner) {
    runner.add(RetryTask::new(HttpServer(state.clone())));
}

#[derive(Clone)]
struct HttpServer(ServerState);

impl task_runner::Task for HttpServer {
    fn start(self, alive: tokio::sync::watch::Receiver<bool>) -> tokio::task::JoinHandle<()> {
        let HttpServer(state) = self;

        let server = Server::bind(&state.config().web.bind)
            .http2_adaptive_window(true)
            .tcp_nodelay(true)
            .serve(make_service_fn(move |socket: &AddrStream| {
                //let remote_addr = socket.get_ref().0.remote_addr();
                let remote_addr = socket.remote_addr();
                let state = state.clone();

                futures::future::ok::<_, std::convert::Infallible>(service_fn(move |req| {
                    let state = state.clone();

                    // gracefully fail and return HTTP 500
                    async move {
                        match tokio::spawn(crate::web::service::service(remote_addr, req, state)).await {
                            Ok(resp) => resp,
                            Err(err) => {
                                log::error!("Internal Server Error: {err}");

                                Ok(StatusCode::INTERNAL_SERVER_ERROR.into_response())
                            }
                        }
                    }
                }))
            }))
            .with_graceful_shutdown(alive.changed().map(|_| ()));

        tokio::spawn(server)
    }
}
