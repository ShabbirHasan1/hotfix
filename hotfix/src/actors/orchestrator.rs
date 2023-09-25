use fefix::tagvalue::{Config, Decoder};
use fefix::Dictionary;
use std::pin::Pin;
use tokio::select;
use tokio::sync::mpsc;
use tokio::time::{sleep, Duration, Instant, Sleep};
use tracing::debug;

use crate::actors::application::{ApplicationHandle, ApplicationMessage};
use crate::actors::socket_writer::WriterHandle;
use crate::config::SessionConfig;
use crate::message::generate_message;
use crate::message::heartbeat::Heartbeat;
use crate::message::logon::{Logon, ResetSeqNumConfig};
use crate::message::parser::RawFixMessage;
use crate::message::FixMessage;
use crate::store::MessageStore;

#[derive(Clone, Debug)]
pub enum OrchestratorMessage<M> {
    FixMessageReceived(RawFixMessage),
    SendHeartbeat,
    SendLogon,
    SendMessage(M),
}

#[derive(Clone)]
pub struct OrchestratorHandle<M> {
    sender: mpsc::Sender<OrchestratorMessage<M>>,
}

impl<M: FixMessage> OrchestratorHandle<M> {
    pub fn new(
        config: SessionConfig,
        writer: WriterHandle,
        application: ApplicationHandle<M>,
        store: impl MessageStore + Send + Sync + 'static,
    ) -> Self {
        let (sender, mailbox) = mpsc::channel::<OrchestratorMessage<M>>(10);
        let actor = OrchestratorActor::new(mailbox, config, writer, application, store);
        tokio::spawn(run_orchestrator(actor));

        Self { sender }
    }

    pub async fn new_fix_message_received(&self, msg: RawFixMessage) {
        self.sender
            .send(OrchestratorMessage::FixMessageReceived(msg))
            .await
            .expect("be able to receive message");
    }

    pub async fn send_message(&self, msg: M) {
        self.sender
            .send(OrchestratorMessage::SendMessage(msg))
            .await
            .expect("message to send successfully");
    }
}

struct OrchestratorActor<M, S> {
    mailbox: mpsc::Receiver<OrchestratorMessage<M>>,
    config: SessionConfig,
    writer: WriterHandle,
    application: ApplicationHandle<M>,
    store: S,
    heartbeat_timer: Pin<Box<Sleep>>,
}

impl<M: FixMessage, S: MessageStore> OrchestratorActor<M, S> {
    fn new(
        mailbox: mpsc::Receiver<OrchestratorMessage<M>>,
        config: SessionConfig,
        writer: WriterHandle,
        application: ApplicationHandle<M>,
        store: S,
    ) -> OrchestratorActor<M, S> {
        let heartbeat_timer = sleep(Duration::from_secs(config.heartbeat_interval));
        Self {
            mailbox,
            config,
            writer,
            application,
            store,
            heartbeat_timer: Box::pin(heartbeat_timer),
        }
    }

    fn decode_message(data: &[u8]) -> M {
        let mut decoder = Decoder::<Config>::new(Dictionary::fix44());
        let msg = decoder.decode(data).expect("decodable FIX message");
        M::parse(msg)
    }

    fn reset_timer(&mut self) {
        let deadline = Instant::now() + Duration::from_secs(self.config.heartbeat_interval);
        self.heartbeat_timer.as_mut().reset(deadline);
    }

    async fn send_message(&mut self, message: impl FixMessage) {
        let seq_num = self.store.next_sender_seq_number().await;
        self.store.increment_sender_seq_number().await;

        let msg = generate_message(
            &self.config.sender_comp_id,
            &self.config.target_comp_id,
            seq_num as usize,
            message,
        );
        self.writer.send_raw_message(RawFixMessage::new(msg)).await;
        self.reset_timer();
    }

    async fn handle(&mut self, message: OrchestratorMessage<M>) {
        match message {
            OrchestratorMessage::FixMessageReceived(fix_message) => {
                debug!("received message: {}", fix_message);
                let decoded_message = Self::decode_message(fix_message.as_bytes());
                let app_message = ApplicationMessage::ReceivedMessage(decoded_message);
                self.store.increment_target_seq_number().await;
                self.application.send_message(app_message).await;
            }
            OrchestratorMessage::SendHeartbeat => {
                self.send_message(Heartbeat {}).await;
            }
            OrchestratorMessage::SendLogon => {
                let reset_config = if self.config.reset_on_logon {
                    self.store.reset().await;
                    ResetSeqNumConfig::Reset
                } else {
                    ResetSeqNumConfig::NoReset(Some(self.store.next_target_seq_number().await))
                };
                let logon = Logon::new(self.config.heartbeat_interval, reset_config);

                self.send_message(logon).await;
            }
            OrchestratorMessage::SendMessage(message) => {
                self.send_message(message).await;
            }
        }
    }
}

async fn run_orchestrator<M, S>(mut actor: OrchestratorActor<M, S>)
where
    M: FixMessage,
    S: MessageStore + Send + 'static,
{
    actor.handle(OrchestratorMessage::SendLogon).await;

    loop {
        let next_message = actor.mailbox.recv();

        select! {
            next = next_message => {
                match next {
                    Some(msg) => {
                        actor.handle(msg).await
                    }
                    None => break,
                }
            }
            () = &mut actor.heartbeat_timer.as_mut() => {
                actor.handle(OrchestratorMessage::SendHeartbeat).await
            }
        }
    }
}
