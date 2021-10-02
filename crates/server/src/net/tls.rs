use std::path::Path;
use std::{fs, io, sync::Arc};

use rustls::internal::pemfile;
use tokio_rustls::rustls::sign::CertifiedKey;
use tokio_rustls::server::TlsStream;
use tokio_rustls::{rustls, TlsAcceptor};

use crate::config::LanternConfig;

#[derive(Debug, thiserror::Error)]
pub enum TlsConfigError {
    #[error("IO Error: {0}")]
    Io(#[from] io::Error),

    #[error("TLS Error: {0}")]
    TlsError(#[from] rustls::TLSError),

    #[error("Key Extract Error")]
    KeyExtractError,

    #[error("Multiple Keys")]
    MultipleKeys,

    #[error("Cert Extract Error")]
    CertExtractError,
}

// Load public certificate from file.
fn load_certs(filename: &Path) -> Result<Vec<rustls::Certificate>, TlsConfigError> {
    // Open certificate file.
    let mut reader = io::BufReader::new(fs::File::open(filename)?);

    // Load and return certificate.
    pemfile::certs(&mut reader).map_err(|_| TlsConfigError::CertExtractError)
}

// Load private key from file.
fn load_private_key(filename: &Path) -> Result<rustls::PrivateKey, TlsConfigError> {
    // Open keyfile.
    let keyfile = fs::File::open(filename)?;
    let mut reader = io::BufReader::new(keyfile);

    // Load and return a single private key.
    let keys = pemfile::pkcs8_private_keys(&mut reader).map_err(|_| TlsConfigError::KeyExtractError)?;

    if keys.len() != 1 {
        return Err(TlsConfigError::MultipleKeys);
    }

    Ok(keys[0].clone())
}

fn load_certified_key(domain: &str, config: &LanternConfig) -> Result<CertifiedKey, TlsConfigError> {
    let mut cert_path = config.cert_path.clone();
    let mut key_path = config.key_path.clone();

    cert_path.push(domain);
    cert_path.push("fullchain.pem");

    key_path.push(domain);
    key_path.push("privkey.pem");

    let cert = load_certs(&cert_path)?;
    let key = load_private_key(&key_path)?;

    let signing_key = rustls::sign::any_supported_type(&key).unwrap();

    Ok(CertifiedKey::new(cert, Arc::new(signing_key)))
}

const DOMAINS: &[&str] = &["lantern.chat", "cdn.lanternchat.net"];

pub fn load_config(config: &LanternConfig) -> Result<rustls::ServerConfig, TlsConfigError> {
    let mut cfg = rustls::ServerConfig::new(rustls::NoClientAuth::new());

    let mut resolver = ResolvesServerCertUsingSNI::new();
    for domain in DOMAINS {
        resolver.add(domain, load_certified_key(domain, config)?)?;
    }

    cfg.cert_resolver = Arc::new(resolver);

    cfg.set_protocols(&[b"h2".to_vec(), b"http/1.1".to_vec()]);

    Ok(cfg)
}

use hashbrown::HashMap;

pub struct ResolvesServerCertUsingSNI {
    by_name: HashMap<String, rustls::sign::CertifiedKey>,
}

use rustls::TLSError;

impl ResolvesServerCertUsingSNI {
    /// Create a new and empty (ie, knows no certificates) resolver.
    pub fn new() -> ResolvesServerCertUsingSNI {
        ResolvesServerCertUsingSNI {
            by_name: HashMap::new(),
        }
    }

    /// Add a new `sign::CertifiedKey` to be used for the given SNI `name`.
    ///
    /// This function fails if `name` is not a valid DNS name, or if
    /// it's not valid for the supplied certificate, or if the certificate
    /// chain is syntactically faulty.
    pub fn add(&mut self, name: &str, ck: rustls::sign::CertifiedKey) -> Result<(), TLSError> {
        let checked_name = tokio_rustls::webpki::DNSNameRef::try_from_ascii_str(name)
            .map_err(|_| TLSError::General("Bad DNS name".into()))?;

        ck.cross_check_end_entity_cert(Some(checked_name))?;
        self.by_name.insert(name.into(), ck);
        Ok(())
    }
}

impl rustls::ResolvesServerCert for ResolvesServerCertUsingSNI {
    fn resolve(&self, client_hello: rustls::ClientHello) -> Option<rustls::sign::CertifiedKey> {
        if let Some(name) = client_hello.server_name() {
            log::trace!("Server name: {:?}", name);
            self.by_name.get(name.into()).cloned()
        } else {
            log::trace!("NO SERVER NAME GIVEN");
            // This kind of resolver requires SNI
            None
        }
    }
}
