use tokio::io::{AsyncWrite, AsyncWriteExt, WriteHalf};
use tokio::sync::mpsc;
use tracing::debug;

use crate::message::parser::RawFixMessage;

#[derive(Clone, Debug)]
pub enum WriterMessage {
    SendMessage(RawFixMessage),
}

#[derive(Clone, Debug)]
pub struct WriterRef {
    sender: mpsc::Sender<WriterMessage>,
}

impl WriterRef {
    pub fn new(writer: WriteHalf<impl AsyncWrite + Send + 'static>) -> Self {
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

struct WriterActor<W> {
    writer: WriteHalf<W>,
    mailbox: mpsc::Receiver<WriterMessage>,
}

impl<W: AsyncWrite> WriterActor<W> {
    fn new(writer: WriteHalf<W>, mailbox: mpsc::Receiver<WriterMessage>) -> Self {
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

async fn run_writer<W: AsyncWrite>(mut actor: WriterActor<W>) {
    while let Some(msg) = actor.mailbox.recv().await {
        actor.handle(msg).await;
    }

    debug!("writer loop is shutting down");
}
