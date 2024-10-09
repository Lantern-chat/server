//! QUIC-specific TLS utilities.

use std::{io, path::Path};

use rustls::{
    pki_types::{CertificateDer, PrivateKeyDer, PrivatePkcs8KeyDer},
    ClientConfig, RootCertStore, ServerConfig,
};

pub fn alpn_protocols() -> Vec<Vec<u8>> {
    vec![b"lc".to_vec()]
}

pub async fn read_key(key_path: &Path) -> Result<PrivateKeyDer<'static>, io::Error> {
    let key = tokio::fs::read(key_path).await?;

    if matches!(key_path.extension(), Some(x) if x == "der") {
        Ok(PrivateKeyDer::Pkcs8(PrivatePkcs8KeyDer::from(key)))
    } else {
        rustls_pemfile::private_key(&mut &*key)?
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "no keys found in key file"))
    }
}

pub async fn read_cert(cert_path: &Path) -> Result<Vec<CertificateDer<'static>>, io::Error> {
    let cert = tokio::fs::read(cert_path).await?;

    if matches!(cert_path.extension(), Some(x) if x == "der") {
        Ok(vec![CertificateDer::from(cert)])
    } else {
        rustls_pemfile::certs(&mut &*cert).collect::<Result<_, _>>()
    }
}

pub async fn server_config(key_path: &Path, cert_path: &Path) -> Result<ServerConfig, io::Error> {
    let (key, certs) = tokio::try_join!(read_key(key_path), read_cert(cert_path))?;

    // TLS 1.3 only for QUIC
    let mut config = ServerConfig::builder_with_protocol_versions(&[&rustls::version::TLS13])
        .with_no_client_auth()
        .with_single_cert(certs, key)
        .map_err(|e| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!("failed to build server config: {}", e),
            )
        })?;

    config.alpn_protocols = alpn_protocols();
    config.max_early_data_size = u32::MAX;

    Ok(config)
}

pub async fn client_config(cert_path: &Path) -> Result<ClientConfig, io::Error> {
    let mut roots = RootCertStore::empty();

    for cert in read_cert(cert_path).await? {
        roots.add(cert).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
    }

    // TLS 1.3 only for QUIC
    let mut config = ClientConfig::builder_with_protocol_versions(&[&rustls::version::TLS13])
        .with_root_certificates(roots)
        .with_no_client_auth();

    config.alpn_protocols = alpn_protocols();

    Ok(config)
}
