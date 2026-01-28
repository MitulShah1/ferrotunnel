//! TLS transport using rustls

use super::socket_tuning::configure_socket_silent;
use super::BoxedStream;
use rustls::pki_types::{CertificateDer, PrivateKeyDer, ServerName};
use rustls::{ClientConfig, RootCertStore, ServerConfig};
use rustls_pemfile::{certs, private_key};
use std::fs::File;
use std::io::{self, BufReader, ErrorKind};
use std::path::Path;
use std::sync::Arc;
use tokio::net::TcpStream;
use tokio_rustls::{TlsAcceptor, TlsConnector};

#[derive(Debug, Clone)]
pub struct TlsTransportConfig {
    pub ca_cert_path: Option<String>,
    pub cert_path: String,
    pub key_path: String,
    pub server_name: Option<String>,
    pub client_auth: bool,
}

fn load_certs(path: &Path) -> io::Result<Vec<CertificateDer<'static>>> {
    let file = File::open(path)?;
    let mut reader = BufReader::new(file);
    certs(&mut reader)
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| io::Error::new(ErrorKind::InvalidData, e))
}

fn load_private_key(path: &Path) -> io::Result<PrivateKeyDer<'static>> {
    let file = File::open(path)?;
    let mut reader = BufReader::new(file);
    private_key(&mut reader)?
        .ok_or_else(|| io::Error::new(ErrorKind::InvalidData, "no private key found"))
}

pub fn create_client_config(config: &TlsTransportConfig) -> io::Result<Arc<ClientConfig>> {
    let mut root_store = RootCertStore::empty();

    if let Some(ca_path) = &config.ca_cert_path {
        let ca_certs = load_certs(Path::new(ca_path))?;
        for cert in ca_certs {
            root_store.add(cert).map_err(|e| {
                io::Error::new(ErrorKind::InvalidData, format!("invalid CA cert: {e}"))
            })?;
        }
    } else {
        return Err(io::Error::new(
            ErrorKind::InvalidInput,
            "CA certificate path required for TLS",
        ));
    }

    let client_config = ClientConfig::builder()
        .with_root_certificates(root_store)
        .with_no_client_auth();

    Ok(Arc::new(client_config))
}

pub fn create_server_config(config: &TlsTransportConfig) -> io::Result<Arc<ServerConfig>> {
    let certs = load_certs(Path::new(&config.cert_path))?;
    let key = load_private_key(Path::new(&config.key_path))?;

    let server_config = ServerConfig::builder()
        .with_no_client_auth()
        .with_single_cert(certs, key)
        .map_err(|e| io::Error::new(ErrorKind::InvalidData, format!("TLS config error: {e}")))?;

    Ok(Arc::new(server_config))
}

#[allow(clippy::expect_used)]
pub async fn connect(addr: &str, config: &TlsTransportConfig) -> io::Result<BoxedStream> {
    let client_config = create_client_config(config)?;
    let connector = TlsConnector::from(client_config);

    let tcp_stream = TcpStream::connect(addr).await?;
    configure_socket_silent(&tcp_stream);

    let server_name = config
        .server_name
        .as_ref()
        .map(|s| ServerName::try_from(s.clone()))
        .transpose()
        .map_err(|e| io::Error::new(ErrorKind::InvalidInput, format!("invalid server name: {e}")))?
        .unwrap_or_else(|| {
            let host = addr.split(':').next().unwrap_or("localhost");
            ServerName::try_from(host.to_string()).unwrap_or_else(|_| {
                ServerName::try_from("localhost".to_string()).expect("localhost is valid")
            })
        });

    let tls_stream = connector.connect(server_name, tcp_stream).await?;
    Ok(Box::pin(tls_stream))
}

pub async fn accept_tls(
    tcp_stream: TcpStream,
    config: &TlsTransportConfig,
) -> io::Result<tokio_rustls::server::TlsStream<TcpStream>> {
    let server_config = create_server_config(config)?;
    let acceptor = TlsAcceptor::from(server_config);
    acceptor.accept(tcp_stream).await
}
