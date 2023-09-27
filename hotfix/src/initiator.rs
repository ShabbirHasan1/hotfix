use tokio::io::{AsyncRead, AsyncWrite};

use crate::actors::application::{Application, ApplicationRef};
use crate::actors::session::SessionRef;
use crate::actors::socket_reader::ReaderRef;
use crate::actors::socket_writer::WriterRef;
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
    _writer: WriterRef,
    _reader: ReaderRef,
    orchestrator: SessionRef<M>,
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

    let application_handle = ApplicationRef::new(application);
    let writer_handle = WriterRef::new(writer);
    let orchestrator_handle =
        SessionRef::new(config, writer_handle.clone(), application_handle, store);
    let reader_handle = ReaderRef::new(reader, orchestrator_handle.clone());

    FixConnection {
        _writer: writer_handle,
        _reader: reader_handle,
        orchestrator: orchestrator_handle,
    }
}
