use tokio::net::TcpStream;

use crate::config::SessionConfig;

pub async fn create_tcp_connection(session_config: &SessionConfig) -> TcpStream {
    let address = format!(
        "{}:{}",
        &session_config.connection_host, &session_config.connection_port
    );
    TcpStream::connect(address).await.unwrap()
}
