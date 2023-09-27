use fefix::tagvalue::{Config, Decoder};
use fefix::Dictionary;
use std::pin::Pin;
use tokio::select;
use tokio::sync::mpsc;
use tokio::time::{sleep, Duration, Instant, Sleep};
use tracing::{debug, error, warn};

use crate::actors::application::{ApplicationMessage, ApplicationRef};
use crate::actors::socket_writer::WriterRef;
use crate::config::SessionConfig;
use crate::message::generate_message;
use crate::message::heartbeat::Heartbeat;
use crate::message::logon::{Logon, ResetSeqNumConfig};
use crate::message::parser::RawFixMessage;
use crate::message::FixMessage;
use crate::store::MessageStore;

#[derive(Clone, Debug)]
pub enum SessionMessage<M> {
    FixMessageReceived(RawFixMessage),
    SendHeartbeat,
    SendMessage(M),
    Disconnected(String),
    RegisterWriter(WriterRef),
}

#[derive(Clone)]
pub struct SessionRef<M> {
    sender: mpsc::Sender<SessionMessage<M>>,
}

impl<M: FixMessage> SessionRef<M> {
    pub fn new(
        config: SessionConfig,
        application: ApplicationRef<M>,
        store: impl MessageStore + Send + Sync + 'static,
    ) -> Self {
        let (sender, mailbox) = mpsc::channel::<SessionMessage<M>>(10);
        let actor = SessionActor::new(mailbox, config, None, application, store);
        tokio::spawn(run_session(actor));

        Self { sender }
    }

    pub async fn register_writer(&self, writer: WriterRef) {
        self.sender
            .send(SessionMessage::RegisterWriter(writer))
            .await
            .expect("be able to register writer");
    }

    pub async fn new_fix_message_received(&self, msg: RawFixMessage) {
        self.sender
            .send(SessionMessage::FixMessageReceived(msg))
            .await
            .expect("be able to receive message");
    }

    pub async fn disconnect(&self, reason: String) {
        self.sender
            .send(SessionMessage::Disconnected(reason))
            .await
            .expect("be able to send disconnect");
    }

    pub async fn send_message(&self, msg: M) {
        self.sender
            .send(SessionMessage::SendMessage(msg))
            .await
            .expect("message to send successfully");
    }
}

struct SessionActor<M, S> {
    mailbox: mpsc::Receiver<SessionMessage<M>>,
    config: SessionConfig,
    writer: Option<WriterRef>,
    application: ApplicationRef<M>,
    store: S,
    heartbeat_timer: Pin<Box<Sleep>>,
    disconnected: bool,
}

impl<M: FixMessage, S: MessageStore> SessionActor<M, S> {
    fn new(
        mailbox: mpsc::Receiver<SessionMessage<M>>,
        config: SessionConfig,
        writer: Option<WriterRef>,
        application: ApplicationRef<M>,
        store: S,
    ) -> SessionActor<M, S> {
        let heartbeat_timer = sleep(Duration::from_secs(config.heartbeat_interval));
        Self {
            mailbox,
            config,
            writer,
            application,
            store,
            heartbeat_timer: Box::pin(heartbeat_timer),
            disconnected: false,
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
        match self.writer {
            None => {
                error!("trying to write without an established connection");
            }
            Some(ref w) => {
                w.send_raw_message(RawFixMessage::new(msg)).await;
                self.reset_timer();
            }
        }
    }

    async fn send_logon(&mut self) {
        let reset_config = if self.config.reset_on_logon {
            self.store.reset().await;
            ResetSeqNumConfig::Reset
        } else {
            ResetSeqNumConfig::NoReset(Some(self.store.next_target_seq_number().await))
        };
        let logon = Logon::new(self.config.heartbeat_interval, reset_config);

        self.send_message(logon).await;
    }

    async fn handle(&mut self, message: SessionMessage<M>) {
        match message {
            SessionMessage::FixMessageReceived(fix_message) => {
                debug!("received message: {}", fix_message);
                let decoded_message = Self::decode_message(fix_message.as_bytes());
                let app_message = ApplicationMessage::ReceivedMessage(decoded_message);
                self.store.increment_target_seq_number().await;
                self.application.send_message(app_message).await;
            }
            SessionMessage::SendHeartbeat => {
                self.send_message(Heartbeat {}).await;
            }
            SessionMessage::SendMessage(message) => {
                self.send_message(message).await;
            }
            SessionMessage::Disconnected(reason) => {
                warn!("disconnected from peer: {reason}");
                self.application.send_logout(reason).await;
                self.disconnected = true;
            }
            SessionMessage::RegisterWriter(w) => {
                self.writer = Some(w);
                self.send_logon().await;
            }
        }
    }
}

async fn run_session<M, S>(mut actor: SessionActor<M, S>)
where
    M: FixMessage,
    S: MessageStore + Send + 'static,
{
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
                actor.handle(SessionMessage::SendHeartbeat).await
            }
        }
    }

    debug!("session is shutting down")
}
