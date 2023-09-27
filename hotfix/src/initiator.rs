use tokio::io::{AsyncRead, AsyncWrite};

use crate::actors::application::{Application, ApplicationHandle};
use crate::actors::session::SessionHandle;
use crate::actors::socket_reader::ReaderHandle;
use crate::actors::socket_writer::WriterHandle;
use crate::config::SessionConfig;
use crate::message::FixMessage;
use crate::store::MessageStore;
use crate::transport::{create_tcp_connection, create_tcp_over_tls_connection};

pub struct Initiator<M> {
    pub config: SessionConfig,
    connection: FixConnection<M>,
}

impl<M: FixMessage> Initiator<M> {
    pub async fn new(
        config: SessionConfig,
        application: impl Application<M>,
        store: impl MessageStore + Send + Sync + 'static,
    ) -> Self {
        let spawned_config = config.clone();
        let connection = establish_connection(spawned_config, application, store).await;

        Self { config, connection }
    }

    pub async fn send_message(&self, msg: M) {
        self.connection.orchestrator.send_message(msg).await;
    }

    pub fn is_interested(&self, sender_comp_id: &str, target_comp_id: &str) -> bool {
        self.config.sender_comp_id == sender_comp_id && self.config.target_comp_id == target_comp_id
    }
}

struct FixConnection<M> {
    // we hold on to the writer and reader so they're not dropped prematurely
    _writer: WriterHandle,
    _reader: ReaderHandle,
    orchestrator: SessionHandle<M>,
}

async fn establish_connection<M: FixMessage>(
    config: SessionConfig,
    application: impl Application<M>,
    store: impl MessageStore + Send + Sync + 'static,
) -> FixConnection<M> {
    let use_tls = config.tls_config.is_some();
    if use_tls {
        let stream = create_tcp_over_tls_connection(&config).await;
        _establish_connection(stream, config, application, store).await
    } else {
        let stream = create_tcp_connection(&config).await;
        _establish_connection(stream, config, application, store).await
    }
}

async fn _establish_connection<M, Stream>(
    stream: Stream,
    config: SessionConfig,
    application: impl Application<M>,
    store: impl MessageStore + Send + Sync + 'static,
) -> FixConnection<M>
where
    M: FixMessage,
    Stream: AsyncRead + AsyncWrite + Send + 'static,
{
    let (reader, writer) = tokio::io::split(stream);

    let application_handle = ApplicationHandle::new(application);
    let writer_handle = WriterHandle::new(writer);
    let orchestrator_handle =
        SessionHandle::new(config, writer_handle.clone(), application_handle, store);
    let reader_handle = ReaderHandle::new(reader, orchestrator_handle.clone());

    FixConnection {
        _writer: writer_handle,
        _reader: reader_handle,
        orchestrator: orchestrator_handle,
    }
}
