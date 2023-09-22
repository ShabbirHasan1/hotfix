use fefix::tagvalue::{Config, Decoder};
use fefix::Dictionary;
use tokio::select;
use tokio::sync::mpsc;
use tokio::time::{sleep, Duration, Instant};
use tracing::debug;

use crate::actors::application::{ApplicationHandle, ApplicationMessage};
use crate::actors::socket_writer::WriterHandle;
use crate::config::SessionConfig;
use crate::message::generate_message;
use crate::message::heartbeat::Heartbeat;
use crate::message::logon::Logon;
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

struct HandleOutput {
    reset_heartbeat: bool,
}

impl HandleOutput {
    fn new(reset_heartbeat: bool) -> Self {
        Self { reset_heartbeat }
    }
}

struct OrchestratorActor<M, S> {
    mailbox: mpsc::Receiver<OrchestratorMessage<M>>,
    config: SessionConfig,
    writer: WriterHandle,
    application: ApplicationHandle<M>,
    store: S,
}

impl<M: FixMessage, S: MessageStore> OrchestratorActor<M, S> {
    fn new(
        mailbox: mpsc::Receiver<OrchestratorMessage<M>>,
        config: SessionConfig,
        writer: WriterHandle,
        application: ApplicationHandle<M>,
        store: S,
    ) -> OrchestratorActor<M, S> {
        Self {
            mailbox,
            config,
            writer,
            application,
            store,
        }
    }

    fn decode_message(data: &[u8]) -> M {
        let mut decoder = Decoder::<Config>::new(Dictionary::fix44());
        let msg = decoder.decode(data).expect("decodable FIX message");
        M::parse(msg)
    }

    async fn handle(&mut self, message: OrchestratorMessage<M>) -> HandleOutput {
        match message {
            OrchestratorMessage::FixMessageReceived(fix_message) => {
                debug!("received message: {}", fix_message);
                let decoded_message = Self::decode_message(fix_message.as_bytes());
                let app_message = ApplicationMessage::ReceivedMessage(decoded_message);
                self.application.send_message(app_message).await;
            }
            OrchestratorMessage::SendHeartbeat => {
                let seq_num = self.store.next_sender_seq_number().await;
                self.store.increment_sender_seq_number().await;

                let msg = generate_message(
                    &self.config.sender_comp_id,
                    &self.config.target_comp_id,
                    seq_num as usize,
                    Heartbeat {},
                );
                self.writer.send_raw_message(RawFixMessage::new(msg)).await;
                return HandleOutput::new(true);
            }
            OrchestratorMessage::SendLogon => {
                let seq_num = self.store.next_sender_seq_number().await;
                self.store.increment_sender_seq_number().await;

                let logon = Logon::new(self.config.heartbeat_interval);
                let msg = generate_message(
                    &self.config.sender_comp_id,
                    &self.config.target_comp_id,
                    seq_num as usize,
                    logon,
                );
                self.writer.send_raw_message(RawFixMessage::new(msg)).await;
                return HandleOutput::new(true);
            }
            OrchestratorMessage::SendMessage(msg) => {
                let seq_num = self.store.next_sender_seq_number().await;
                self.store.increment_sender_seq_number().await;

                let raw_message = generate_message(
                    &self.config.sender_comp_id,
                    &self.config.target_comp_id,
                    seq_num as usize,
                    msg,
                );
                self.writer
                    .send_raw_message(RawFixMessage::new(raw_message))
                    .await;
                return HandleOutput::new(true);
            }
        }

        HandleOutput::new(false)
    }
}

async fn run_orchestrator<M, S>(mut actor: OrchestratorActor<M, S>)
where
    M: FixMessage,
    S: MessageStore + Send + 'static,
{
    actor.handle(OrchestratorMessage::SendLogon).await;
    let next_heartbeat = sleep(Duration::from_secs(actor.config.heartbeat_interval));
    tokio::pin!(next_heartbeat);

    loop {
        let next_message = actor.mailbox.recv();

        let outcome = select! {
            next = next_message => {
                match next {
                    Some(msg) => {
                        actor.handle(msg).await
                    }
                    None => break,
                }
            }
            () = &mut next_heartbeat => {
                actor.handle(OrchestratorMessage::SendHeartbeat).await
            }
        };

        if outcome.reset_heartbeat {
            let deadline = Instant::now() + Duration::from_secs(actor.config.heartbeat_interval);
            next_heartbeat.as_mut().reset(deadline);
        }
    }
}
