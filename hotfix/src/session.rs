use crate::actors::orchestrator::OrchestratorHandle;
use crate::actors::socket_reader::ReaderHandle;
use crate::actors::socket_writer::WriterHandle;
use crate::config::SessionConfig;
use crate::message::hardcoded::FixMessage;
use crate::tls_client::Client;

pub struct Session {
    pub config: SessionConfig,
    connection: FixConnection,
}

impl Session {
    pub async fn new(config: SessionConfig) -> Self {
        let spawned_config = config.clone();
        let connection = establish_connection(spawned_config).await;

        Self { config, connection }
    }

    pub async fn send_message(&self, msg: FixMessage) {
        self.connection.orchestrator.send_message(msg).await;
    }

    pub fn is_interested(&self, sender_comp_id: &str, target_comp_id: &str) -> bool {
        self.config.sender_comp_id == sender_comp_id && self.config.target_comp_id == target_comp_id
    }
}

struct FixConnection {
    writer: WriterHandle,
    reader: ReaderHandle,
    orchestrator: OrchestratorHandle,
}

async fn establish_connection(config: SessionConfig) -> FixConnection {
    let tls_client = Client::new(&config).await;

    let (reader, writer) = tls_client.split();

    let writer_handle = WriterHandle::new(writer);
    let orchestrator_handle = OrchestratorHandle::new(config, writer_handle.clone());
    let reader_handle = ReaderHandle::new(reader, orchestrator_handle.clone());

    FixConnection {
        writer: writer_handle,
        reader: reader_handle,
        orchestrator: orchestrator_handle,
    }
}
