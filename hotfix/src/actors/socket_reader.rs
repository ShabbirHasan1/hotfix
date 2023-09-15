use crate::actors::orchestrator::OrchestratorHandle;
use crate::message::parser::Parser;
use tokio::io::{AsyncReadExt, ReadHalf};
use tokio::sync::mpsc;

use crate::tls_client::FixStream;

#[derive(Clone, Debug)]
pub struct ReaderMessage;

pub struct ReaderHandle {
    sender: mpsc::Sender<ReaderMessage>,
}

impl ReaderHandle {
    pub fn new(reader: ReadHalf<FixStream>, orchestrator: OrchestratorHandle) -> Self {
        let (sender, mailbox) = mpsc::channel(10);
        let actor = ReaderActor::new(reader, mailbox, orchestrator);
        tokio::spawn(run_reader(actor));

        Self { sender }
    }
}

struct ReaderActor {
    reader: ReadHalf<FixStream>,
    mailbox: mpsc::Receiver<ReaderMessage>,
    orchestrator: OrchestratorHandle,
}

impl ReaderActor {
    fn new(
        reader: ReadHalf<FixStream>,
        mailbox: mpsc::Receiver<ReaderMessage>,
        orchestrator: OrchestratorHandle,
    ) -> Self {
        Self {
            reader,
            mailbox,
            orchestrator,
        }
    }
}

async fn run_reader(mut actor: ReaderActor) {
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
