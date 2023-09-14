use crate::actors::orchestrator::OrchestratorHandle;
use crate::actors::socket_reader::reader_loop;
use crate::actors::socket_writer::WriterHandle;
use crate::config::SessionConfig;
use crate::message::logon;
use crate::message::parser::RawFixMessage;
use crate::tls_client::Client;

pub struct Session {
    pub config: SessionConfig,
}

impl Session {
    pub fn new(config: SessionConfig) -> Self {
        let spawned_config = config.clone();
        tokio::spawn(async move {
            establish_connection(spawned_config).await;
        });
        Self { config }
    }
}

async fn establish_connection(config: SessionConfig) {
    let tls_client = Client::new(&config).await;

    let (reader, writer) = tls_client.split();
    let logon_message = logon::create_login_message(&config.sender_comp_id, &config.target_comp_id);

    let writer = WriterHandle::new(writer);
    let orchestrator = OrchestratorHandle::new();
    let fut_reader = reader_loop(reader, orchestrator);

    writer
        .send_raw_message(RawFixMessage::new(logon_message))
        .await;

    fut_reader.await;
}
