use std::fs;
use std::io::BufReader;
use std::sync::Arc;

use pki_types::CertificateDer;
use rustls::ClientConfig;
use rustls::{RootCertStore, ServerName};
use tokio::io::{AsyncRead, AsyncWrite, ReadHalf, WriteHalf};
use tokio::net::TcpStream;
use tokio_rustls::{client::TlsStream, TlsConnector};

use crate::config::SessionConfig;

pub type FixStream = TlsStream<TcpStream>;

pub struct Client {
    stream: FixStream,
}

impl Client {
    pub async fn new(session_config: &SessionConfig) -> Self {
        let client_config = Self::get_client_config(session_config);
        let socket = Self::get_socket(session_config).await;
        let stream = wrap_stream(
            socket,
            session_config.connection_host.clone(),
            Arc::new(client_config),
        )
        .await;

        Self { stream }
    }

    pub fn split(self) -> (ReadHalf<FixStream>, WriteHalf<FixStream>) {
        tokio::io::split(self.stream)
    }

    fn get_client_config(session_config: &SessionConfig) -> ClientConfig {
        let root_store = Self::get_root_store(&session_config.ca_certificate_path);
        ClientConfig::builder()
            .with_safe_defaults()
            .with_root_certificates(root_store)
            .with_no_client_auth()
    }

    fn get_root_store(ca_certificate_path: &str) -> RootCertStore {
        let mut root_store = RootCertStore::empty();
        let certs = Self::load_certs(ca_certificate_path);
        root_store.add_parsable_certificates(&certs);

        root_store
    }

    fn load_certs(filename: &str) -> Vec<CertificateDer<'static>> {
        let certfile = fs::File::open(filename).expect("certificate file to be open");
        let mut reader = BufReader::new(certfile);
        rustls_pemfile::certs(&mut reader)
            .map(|result| result.unwrap())
            .collect()
    }

    async fn get_socket(session_config: &SessionConfig) -> TcpStream {
        let address = format!(
            "{}:{}",
            &session_config.connection_host, &session_config.connection_port
        );
        TcpStream::connect(address).await.unwrap()
    }
}

pub async fn wrap_stream<S>(socket: S, domain: String, config: Arc<ClientConfig>) -> TlsStream<S>
where
    S: 'static + AsyncRead + AsyncWrite + Send + Unpin,
{
    let domain = ServerName::try_from(domain.as_str()).unwrap();
    let stream = TlsConnector::from(config);
    let connected = stream.connect(domain, socket).await;

    connected.unwrap()
}
