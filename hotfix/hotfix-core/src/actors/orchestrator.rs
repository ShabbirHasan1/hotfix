use tokio::sync::mpsc;
use tracing::debug;

use crate::message::parser::RawFixMessage;

#[derive(Clone, Debug)]
pub enum OrchestratorMessage {
    FixMessageReceived(RawFixMessage),
}

#[derive(Clone)]
pub struct OrchestratorHandle {
    sender: mpsc::Sender<OrchestratorMessage>,
}

impl OrchestratorHandle {
    pub fn new() -> Self {
        let (sender, mailbox) = mpsc::channel(10);
        let actor = OrchestratorActor::new(mailbox);
        tokio::spawn(run_orchestrator(actor));

        Self { sender }
    }

    pub async fn new_fix_message_received(&self, msg: RawFixMessage) {
        self.sender
            .send(OrchestratorMessage::FixMessageReceived(msg))
            .await
            .expect("be able to receive message");
    }
}

impl Default for OrchestratorHandle {
    fn default() -> Self {
        Self::new()
    }
}

struct OrchestratorActor {
    mailbox: mpsc::Receiver<OrchestratorMessage>,
}

impl OrchestratorActor {
    fn new(mailbox: mpsc::Receiver<OrchestratorMessage>) -> OrchestratorActor {
        Self { mailbox }
    }

    fn handle(&self, message: OrchestratorMessage) {
        match message {
            OrchestratorMessage::FixMessageReceived(fix_message) => {
                debug!("received message: {}", fix_message);
            }
        }
    }
}

async fn run_orchestrator(mut actor: OrchestratorActor) {
    while let Some(msg) = actor.mailbox.recv().await {
        actor.handle(msg);
    }
}
