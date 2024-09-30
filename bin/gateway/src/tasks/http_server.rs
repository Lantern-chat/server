use ftl::serve::{accept::NoDelayAcceptor, Server};

use super::*;

pub fn add_http_server_task(state: &ServerState, runner: &TaskRunner) {
    runner.add(RetryTask::new(HttpServer(state.clone())));
}

#[derive(Clone)]
struct HttpServer(ServerState);

impl task_runner::Task for HttpServer {
    fn start(self, alive: Alive) -> tokio::task::JoinHandle<()> {
        tokio::spawn(async move {
            while *alive.borrow() {
                self.clone().run(alive.clone()).await;
            }
        })
    }
}

impl HttpServer {
    async fn run(self, alive: Alive) {
        let HttpServer(state) = self;

        let https_bind_addr = state.config().local.web.bind;
        let mut http_bind_addr = https_bind_addr;

        // HTTPS may use 8083 or 443, so we can use 8000 or 80, respectively
        http_bind_addr.set_port(if https_bind_addr.port() > 8000 { 8000 } else { 80 });

        // setup server with bind address
        let mut server = Server::bind([http_bind_addr]);

        // spawn a task to shutdown the server when the config changes or the server is no longer alive
        // if the config changes, the server will shutdown gracefully and restart
        tokio::spawn({
            let mut alive = alive.clone();
            let handle = server.handle();
            let state = state.clone();

            async move {
                futures::future::select(
                    std::pin::pin!(alive.changed()),
                    std::pin::pin!(state.config.config_change.notified()),
                )
                .await;

                handle.shutdown();
            }
        });

        // and shutdown behavior, which for the HTTP redirect server is immediate
        server.handle().set_shutdown_timeout(Duration::from_secs(0));

        // and configure HTTP parameters
        server
            .http1()
            .writev(true) // helps with TLS performance
            .pipeline_flush(true);

        // create the service stack
        let service = {
            use ftl::layers::{catch_panic::CatchPanic, resp_timing::RespTimingLayer, Layer};
            use ftl::rewrite::{RedirectKind, RewriteService};

            let layer_stack = (
                RespTimingLayer::default(), // logs the time taken to process each request
                CatchPanic::default(),      // spawns each request in a separate task and catches panics
            );

            let kind = if cfg!(debug_assertions) { RedirectKind::Temporary } else { RedirectKind::Permanent };

            layer_stack.layer(RewriteService::new(kind, move |parts| {
                // Authority is automatically inserted by the RewriteService
                let host = parts.extensions.get::<http::uri::Authority>().unwrap();

                // if the HTTPS port is 443, we don't need to include it in the URL
                if https_bind_addr.port() == 443 {
                    format!("https://{}{}", host.host(), parts.uri.path())
                } else {
                    format!("https://{}:{}{}", host.host(), https_bind_addr.port(), parts.uri.path())
                }
            }))
        };

        let handle = server.handle();

        // spawn the server
        tokio::spawn(server.acceptor(NoDelayAcceptor).serve(service));

        // wait for the server to shutdown
        handle.wait().await;
    }
}
