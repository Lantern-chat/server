use ftl::serve::accept::{limited::LimitedTcpAcceptor, NoDelayAcceptor, PeekingAcceptor, TimeoutAcceptor};
use ftl::serve::tls_rustls::{RustlsAcceptor, RustlsConfig};
use ftl::serve::{Server, TlsConfig as _};

use super::*;

pub fn add_https_server_task(state: &GatewayServerState, runner: &TaskRunner) {
    runner.add(RetryTask::new(HttpsServer(state.clone())));
}

#[derive(Clone)]
struct HttpsServer(GatewayServerState);

impl task_runner::Task for HttpsServer {
    fn start(self, alive: Alive) -> tokio::task::JoinHandle<()> {
        tokio::spawn(async move {
            while *alive.borrow() {
                self.clone().run(alive.clone()).await;
            }
        })
    }
}

impl HttpsServer {
    async fn run(self, alive: Alive) {
        let HttpsServer(state) = self;

        let config = state.config();

        let mut cert_path = config.local.web.cert_path.clone();
        let mut key_path = config.local.web.key_path.clone();

        let tls_config =
            RustlsConfig::from_pem_file(&cert_path, &key_path).await.expect("failed to load TLS config");

        let bind_addr = config.local.web.bind;

        // setup server with bind address
        let mut server = Server::bind([bind_addr]);

        tokio::spawn({
            let (handle, mut alive, state, tls_config) =
                (server.handle(), alive.clone(), state.clone(), tls_config.clone());

            async move {
                loop {
                    tokio::select! {
                        _ = alive.changed() => break,
                        _ = state.config.config_change.notified() => {/*reload tls config*/}
                    }

                    let config = state.config();

                    if config.local.web.cert_path != cert_path || config.local.web.key_path != key_path {
                        cert_path = config.local.web.cert_path.clone();
                        key_path = config.local.web.key_path.clone();

                        let new_config = match RustlsConfig::from_pem_file(&cert_path, &key_path).await {
                            Ok(config) => config,
                            Err(e) => {
                                log::error!("failed to reload TLS config: {e}");
                                continue;
                            }
                        };

                        tls_config.reload_from_config(new_config.get_inner());
                    }
                }

                handle.shutdown();
            }
        });

        // and shutdown behavior
        server.handle().set_shutdown_timeout(Duration::from_secs(1));

        // and configure HTTP parameters
        server
            .http1()
            .writev(true) // helps with TLS performance
            .pipeline_flush(true)
            .http2()
            .max_concurrent_streams(Some(400))
            .adaptive_window(true)
            .enable_connect_protocol(); // used for HTTP/2 Websockets

        // create the service stack
        let service = {
            use ftl::layers::{
                catch_panic::CatchPanic,
                cloneable::Cloneable,
                compression::CompressionLayer,
                convert_body::ConvertBody,
                deferred::DeferredEncoding,
                handle_error::HandleErrorLayer,
                limit_req_body::{LimitBodyError, LimitReqBody},
                normalize::Normalize,
                resp_timing::RespTimingLayer,
                Layer, RealIpLayer,
            };

            let layer_stack = (
                RespTimingLayer::default(), // logs the time taken to process each request
                CatchPanic::default(),      // spawns each request in a separate task and catches panics
                Cloneable::default(),       // makes the service layered below it cloneable
                RealIpLayer::default(),     // extracts the real ip from the request
                ConvertBody::default(),     // converts the body to the correct type
                // limits the request body to 10MiB and rejects large bodies
                // and also handles the error by converting it to a response
                (
                    HandleErrorLayer::new(|err| {
                        use ftl::IntoResponse;

                        core::future::ready(match err {
                            LimitBodyError::BodyError(e) => e.into_response(),
                        })
                    }),
                    LimitReqBody::new(10 << 20),
                ),
                CompressionLayer::default(), // compresses responses
                Normalize::default(),        // normalizes the response structure
                DeferredEncoding::default(), // encodes deferred responses
            );

            layer_stack.layer(crate::web::WebService::new(state.clone()))
        };

        #[rustfmt::skip]
        let acceptor = TimeoutAcceptor::new(
            // 10 second timeout for the entire connection accept process
            Duration::from_secs(10),
            // Accept TLS connections with rustls
            RustlsAcceptor::new(tls_config).acceptor(
                // limit the number of connections per IP to 50
                LimitedTcpAcceptor::new(
                    // TCP_NODELAY, and peek at the first byte of the stream
                    PeekingAcceptor(NoDelayAcceptor),
                    50,
                ).with_privacy_mask(true)
            ),
        );

        // run the server
        server.acceptor(acceptor).serve(service).await.expect("HTTPS server failed");
    }
}
