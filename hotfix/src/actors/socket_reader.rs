use tokio::io::{AsyncReadExt, ReadHalf};
use tokio::sync::mpsc;

use crate::actors::orchestrator::OrchestratorHandle;
use crate::message::parser::Parser;
use crate::message::FixMessage;

use crate::tls_client::FixStream;

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
        reader: ReadHalf<FixStream>,
        orchestrator: OrchestratorHandle<M>,
    ) -> Self {
        let (sender, mailbox) = mpsc::channel(10);
        let actor = ReaderActor::new(reader, mailbox, orchestrator);
        tokio::spawn(run_reader(actor));

        Self { sender }
    }
}

struct ReaderActor<M> {
    reader: ReadHalf<FixStream>,
    #[allow(dead_code)]
    mailbox: mpsc::Receiver<ReaderMessage>,
    orchestrator: OrchestratorHandle<M>,
}

impl<M> ReaderActor<M> {
    fn new(
        reader: ReadHalf<FixStream>,
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

async fn run_reader<M: FixMessage>(mut actor: ReaderActor<M>) {
    let mut parser = Parser::default();
    loop {
        let mut buf = vec![];
        actor.reader.read_buf(&mut buf).await.unwrap();
        let messages = parser.parse(&buf);

        for msg in messages {
            actor.orchestrator.new_fix_message_received(msg).await;
        }
    }
}
