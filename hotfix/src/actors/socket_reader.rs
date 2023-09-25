use tokio::io::{AsyncRead, AsyncReadExt, ReadHalf};
use tokio::sync::mpsc;
use tracing::debug;

use crate::actors::orchestrator::OrchestratorHandle;
use crate::message::parser::Parser;
use crate::message::FixMessage;

#[derive(Clone, Debug)]
pub struct ReaderMessage;

pub struct ReaderHandle {
    // not sure we'll need to send messages to the reader,
    // but we're keeping the standard actor structure for now
    #[allow(dead_code)]
    sender: mpsc::Sender<ReaderMessage>,
}

impl ReaderHandle {
    pub fn new<M: FixMessage>(
        reader: ReadHalf<impl AsyncRead + Send + 'static>,
        orchestrator: OrchestratorHandle<M>,
    ) -> Self {
        let (sender, mailbox) = mpsc::channel(10);
        let actor = ReaderActor::new(reader, mailbox, orchestrator);
        tokio::spawn(run_reader(actor));

        Self { sender }
    }
}

struct ReaderActor<M, R> {
    reader: ReadHalf<R>,
    #[allow(dead_code)]
    mailbox: mpsc::Receiver<ReaderMessage>,
    orchestrator: OrchestratorHandle<M>,
}

impl<M, R: AsyncRead> ReaderActor<M, R> {
    fn new(
        reader: ReadHalf<R>,
        mailbox: mpsc::Receiver<ReaderMessage>,
        orchestrator: OrchestratorHandle<M>,
    ) -> Self {
        Self {
            reader,
            mailbox,
            orchestrator,
        }
    }
}

async fn run_reader<M, R>(mut actor: ReaderActor<M, R>)
where
    M: FixMessage,
    R: AsyncRead,
{
    let mut parser = Parser::default();
    loop {
        let mut buf = vec![];

        match actor.reader.read_buf(&mut buf).await {
            Ok(0) => {
                actor
                    .orchestrator
                    .disconnect("received EOF".to_string())
                    .await;
                break;
            }
            Err(err) => {
                actor.orchestrator.disconnect(err.to_string()).await;
                break;
            }
            Ok(_) => {
                let messages = parser.parse(&buf);

                for msg in messages {
                    actor.orchestrator.new_fix_message_received(msg).await;
                }
            }
        }
    }
    debug!("reader loop is shutting down");
}
