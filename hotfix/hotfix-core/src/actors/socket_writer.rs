use tokio::io::{AsyncWriteExt, WriteHalf};
use tokio::sync::mpsc;
use tracing::debug;

use crate::message::parser::RawFixMessage;
use crate::tls_client::FixStream;

#[derive(Clone, Debug)]
pub enum WriterMessage {
    SendMessage(RawFixMessage),
}

#[derive(Clone)]
pub struct WriterHandle {
    sender: mpsc::Sender<WriterMessage>,
}

impl WriterHandle {
    pub fn new(writer: WriteHalf<FixStream>) -> Self {
        let (sender, mailbox) = mpsc::channel(10);
        let actor = WriterActor::new(writer, mailbox);
        tokio::spawn(run_writer(actor));

        Self { sender }
    }

    pub async fn send_raw_message(&self, msg: RawFixMessage) {
        self.sender
            .send(WriterMessage::SendMessage(msg))
            .await
            .expect("be able to send message");
    }
}

struct WriterActor {
    writer: WriteHalf<FixStream>,
    mailbox: mpsc::Receiver<WriterMessage>,
}

impl WriterActor {
    fn new(writer: WriteHalf<FixStream>, mailbox: mpsc::Receiver<WriterMessage>) -> Self {
        Self { writer, mailbox }
    }

    async fn handle(&mut self, message: WriterMessage) {
        match message {
            WriterMessage::SendMessage(fix_message) => {
                self.writer
                    .write_all(fix_message.as_bytes())
                    .await
                    .expect("logon message to send");
                debug!("sent message: {}", fix_message);
            }
        }
    }
}

async fn run_writer(mut actor: WriterActor) {
    while let Some(msg) = actor.mailbox.recv().await {
        actor.handle(msg).await;
    }
}
