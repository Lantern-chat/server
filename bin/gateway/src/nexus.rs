use std::{io, net::Ipv6Addr, sync::Arc};

use quinn::{crypto::rustls::QuicClientConfig, ClientConfig, Endpoint};

use ::rpc::{
    client::{RpcClient, RpcClientError},
    request::RpcRequest,
    stream::RpcRecvReader,
    DeserializeExt,
};
use sdk::api::error::ApiError;

use crate::config::{LocalConfig, SharedConfig};
use crate::prelude::*;

pub async fn connect(config: &LocalConfig) -> io::Result<RpcClient> {
    let tls_config = ::rpc::tls::client_config(&config.rpc.cert_path).await?;

    let quic_config = QuicClientConfig::try_from(Arc::new(tls_config))
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, format!("failed to convert TLS config: {e}")))?;

    // bind local endpoint to `[::]:0`
    let mut endpoint = Endpoint::client((Ipv6Addr::UNSPECIFIED, 0).into())?;

    // setup client config for proper TLS handling
    endpoint.set_default_client_config(ClientConfig::new(Arc::new(quic_config)));

    Ok(RpcClient::new(
        endpoint,
        Snowflake::null(), // Nexus doesn't have a faction ID
        config.rpc.nexus_addr,
        config.rpc.max_conns,
        "Nexus",
    ))
}

pub async fn fetch_shared_config(client: &RpcClient) -> Result<SharedConfig, Error> {
    let mut stream = RpcRecvReader::new(client.send(&RpcRequest::GetSharedConfig).await?);

    match stream.recv::<Result<SharedConfig, ApiError>>().await? {
        Some(config) => match config.deserialize_full() {
            Ok(res) => match res {
                Ok(config) => Ok(config),
                Err(e) => Err(Error::ApiError(e)),
            },
            // TODO: Better error message
            Err(_) => Err(Error::RpcClientError(RpcClientError::EncodingError)),
        },
        None => Err(Error::IOError(io::Error::new(
            io::ErrorKind::UnexpectedEof,
            "no data received",
        ))),
    }
}
