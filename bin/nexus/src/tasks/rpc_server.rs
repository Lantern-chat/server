use std::{io, sync::Arc};

use super::*;

use quinn::crypto::rustls::QuicServerConfig;
use quinn::{Endpoint, ServerConfig};

pub fn add_rpc_server_task(state: &ServerState, runner: &task_runner::TaskRunner) {
    runner.add(RetryTask::new(RpcServer(state.clone())));
}

#[derive(Clone)]
struct RpcServer(ServerState);

impl task_runner::Task for RpcServer {
    fn start(self, alive: Alive) -> tokio::task::JoinHandle<()> {
        tokio::spawn(async move {
            while *alive.borrow() {
                if let Err(e) = self.clone().run(alive.clone()).await {
                    log::error!("rpc server error: {}", e);
                }
            }
        })
    }
}

impl RpcServer {
    async fn run(self, mut alive: Alive) -> Result<(), io::Error> {
        let RpcServer(state) = self;

        let config = state.config();

        let bind_addr = config.local.rpc.bind;
        let key_path = config.local.paths.key_path.as_path();
        let cert_path = config.local.paths.cert_path.as_path();

        let tls_config = rpc::tls::server_config(key_path, cert_path).await?;

        let server_config =
            ServerConfig::with_crypto(Arc::new(QuicServerConfig::try_from(tls_config).map_err(|e| {
                io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!("failed to build quic config: {}", e),
                )
            })?));

        let endpoint = Endpoint::server(server_config, bind_addr)?;

        while let Some(incoming) = tokio::select! {
            biased;
            _ = alive.changed() => None,
            incoming = endpoint.accept() => incoming,
        } {
            let state = state.clone();

            tokio::spawn(async move {
                let connecting = match incoming.accept() {
                    Ok(connecting) => connecting,
                    Err(e) => {
                        log::error!("rpc connecting error: {:?}", e);
                        return;
                    }
                };

                let connection = match connecting.await {
                    Ok(connection) => connection,
                    Err(e) => {
                        log::error!("rpc connection error: {:?}", e);
                        return;
                    }
                };

                tokio::join! {
                    state.gateway.insert_gateway_connection(state.clone(), connection.clone()),
                    state.gateway.insert_rpc_connection(state.clone(), connection.clone()),
                };
            });
        }

        Ok(())
    }
}
