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
    session: SessionRef<M>,
}

impl<M: FixMessage> Initiator<M> {
    pub async fn new(
        config: SessionConfig,
        application: impl Application<M>,
        store: impl MessageStore + Send + Sync + 'static,
    ) -> Self {
        let application_ref = ApplicationRef::new(application);
        let session_ref = SessionRef::new(config.clone(), application_ref, store);

        tokio::spawn({
            let config = config.clone();
            let session_ref = session_ref.clone();
            establish_connection(config, session_ref)
        });

        Self {
            config,
            session: session_ref,
        }
    }

    pub async fn send_message(&self, msg: M) {
        self.session.send_message(msg).await;
    }

    pub fn is_interested(&self, sender_comp_id: &str, target_comp_id: &str) -> bool {
        self.config.sender_comp_id == sender_comp_id && self.config.target_comp_id == target_comp_id
    }
}

struct FixConnection {
    _writer: WriterRef,
    _reader: ReaderRef,
}

async fn establish_connection<M: FixMessage>(
    config: SessionConfig,
    session_ref: SessionRef<M>,
) -> FixConnection {
    loop {
        let use_tls = config.tls_config.is_some();

        let conn = if use_tls {
            let stream = create_tcp_over_tls_connection(&config).await;
            _create_io_refs(session_ref.clone(), stream).await
        } else {
            let stream = create_tcp_connection(&config).await;
            _create_io_refs(session_ref.clone(), stream).await
        };
        session_ref.register_writer(conn._writer).await;
        conn._reader.wait_for_disconnect().await;
    }
}

async fn _create_io_refs<M, Stream>(session_ref: SessionRef<M>, stream: Stream) -> FixConnection
where
    M: FixMessage,
    Stream: AsyncRead + AsyncWrite + Send + 'static,
{
    let (reader, writer) = tokio::io::split(stream);

    let writer_ref = WriterRef::new(writer);
    let reader_ref = ReaderRef::new(reader, session_ref);

    FixConnection {
        _writer: writer_ref,
        _reader: reader_ref,
    }
}
