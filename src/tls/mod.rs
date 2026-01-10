use rustls::pki_types::{CertificateDer, PrivateKeyDer};
use rustls::ServerConfig;
use rustls_pemfile::{certs, private_key};
use std::fs::File;
use std::io::BufReader;
use std::path::Path;
use std::sync::Arc;
use tokio_rustls::TlsAcceptor;

#[derive(Clone)]
pub struct TlsConfig {
    pub cert_path: String,
    pub key_path: String,
    pub alpn_protocols: Vec<Vec<u8>>,
}

impl TlsConfig {
    pub fn new(cert_path: String, key_path: String) -> Self {
        Self {
            cert_path,
            key_path,
            alpn_protocols: vec![b"h2".to_vec(), b"http/1.1".to_vec()],
        }
    }

    pub fn with_alpn(mut self, protocols: Vec<&str>) -> Self {
        self.alpn_protocols = protocols.into_iter().map(|p| p.as_bytes().to_vec()).collect();
        self
    }
}

pub struct TlsProvider {
    acceptor: TlsAcceptor,
}

impl TlsProvider {
    pub fn new(config: &TlsConfig) -> anyhow::Result<Self> {
        let certs = load_certs(&config.cert_path)?;
        let key = load_key(&config.key_path)?;

        let mut server_config = ServerConfig::builder()
            .with_no_client_auth()
            .with_single_cert(certs, key)?;

        server_config.alpn_protocols = config.alpn_protocols.clone();
        server_config.max_early_data_size = 0;

        let acceptor = TlsAcceptor::from(Arc::new(server_config));

        Ok(Self { acceptor })
    }

    pub fn acceptor(&self) -> &TlsAcceptor {
        &self.acceptor
    }
}

impl Clone for TlsProvider {
    fn clone(&self) -> Self {
        Self {
            acceptor: self.acceptor.clone(),
        }
    }
}

fn load_certs(path: &str) -> anyhow::Result<Vec<CertificateDer<'static>>> {
    let file = File::open(path)?;
    let mut reader = BufReader::new(file);
    let certs: Vec<CertificateDer<'static>> = certs(&mut reader)
        .filter_map(|c| c.ok())
        .collect();

    if certs.is_empty() {
        anyhow::bail!("No certificates found in {}", path);
    }

    Ok(certs)
}

fn load_key(path: &str) -> anyhow::Result<PrivateKeyDer<'static>> {
    let file = File::open(path)?;
    let mut reader = BufReader::new(file);

    private_key(&mut reader)?
        .ok_or_else(|| anyhow::anyhow!("No private key found in {}", path))
}
