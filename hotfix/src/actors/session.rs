use fefix::tagvalue::{Config, Decoder, FieldAccess, Message};
use fefix::Dictionary;
use std::pin::Pin;
use tokio::select;
use tokio::sync::{mpsc, oneshot};
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
use crate::session_state::SessionState;
use crate::session_state::SessionState::{Connected, Disconnected, LoggedOut};
use crate::store::MessageStore;

#[derive(Debug)]
pub enum SessionMessage<M> {
    /// Tell the session we have received a new FIX message from the reader.
    FixMessageReceived(RawFixMessage),
    /// Ask the session to send a new heartbeat.
    SendHeartbeat,
    /// Ask the session to send a message from the application.
    SendMessage(M),
    /// Let the session know we've been disconnected.
    Disconnected(String),
    /// Register a new writer connected to the other side.
    RegisterWriter(WriterRef),
    /// Ask the session whether we should attempt to reconnect.
    ShouldReconnect(oneshot::Sender<bool>),
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

    pub async fn should_reconnect(&self) -> bool {
        let (sender, receiver) = oneshot::channel();
        self.sender
            .send(SessionMessage::ShouldReconnect(sender))
            .await
            .unwrap();
        receiver.await.expect("to receive a response")
    }
}

struct SessionActor<M, S> {
    mailbox: mpsc::Receiver<SessionMessage<M>>,
    config: SessionConfig,
    state: SessionState,
    writer: Option<WriterRef>,
    application: ApplicationRef<M>,
    store: S,
    heartbeat_timer: Pin<Box<Sleep>>,
    decoder: Decoder,
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
            state: Connected,
            writer,
            application,
            store,
            heartbeat_timer: Box::pin(heartbeat_timer),
            decoder: Decoder::<Config>::new(Dictionary::fix44()),
        }
    }

    fn decode_message<'a>(&'a mut self, data: &'a [u8]) -> Message<&[u8]> {
        self.decoder.decode(data).expect("decodable FIX message")
    }

    async fn on_incoming(&mut self, message: RawFixMessage) {
        debug!("received message: {}", message);
        self.store.increment_target_seq_number().await;

        let decoded_message = self.decode_message(message.as_bytes());
        let message_type = decoded_message.fv_raw(&35).unwrap();
        match message_type {
            b"0" => {
                // TODO: handle heartbeat
            }
            b"1" => {
                // TODO: handle test request
            }
            b"2" => {
                // TODO: handle resend request
            }
            b"3" => {
                // TODO: handle reject
            }
            b"4" => {
                // TODO: handle sequence reset
            }
            b"5" => {
                self.on_logout().await;
            }
            b"A" => {
                // TODO: handle logon
            }
            _ => {
                let parsed_message = M::parse(decoded_message);
                let app_message = ApplicationMessage::ReceivedMessage(parsed_message);
                self.application.send_message(app_message).await;
            }
        }
    }

    async fn on_disconnect(&mut self, reason: String) {
        match self.state {
            Connected => {
                self.state = Disconnected {
                    reconnect: true,
                    reason,
                }
            }
            LoggedOut { reconnect } => {
                self.state = Disconnected {
                    reconnect,
                    reason: "logged out".to_string(),
                }
            }
            Disconnected { .. } => {
                warn!("disconnect messages was received, but the session is already disconnected")
            }
        }
    }

    async fn on_logout(&mut self) {
        // TODO: reconnect = false isn't always valid, this should be more sophisticated
        self.state = LoggedOut { reconnect: false };
        self.disconnect().await;
        self.application
            .send_logout("peer has logged us out".to_string())
            .await;
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
            }
        }
        self.reset_timer();
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

    async fn disconnect(&self) {
        if let Some(writer) = &self.writer {
            writer.disconnect().await
        }
    }

    async fn handle(&mut self, message: SessionMessage<M>) {
        match message {
            SessionMessage::FixMessageReceived(fix_message) => {
                self.on_incoming(fix_message).await;
            }
            SessionMessage::SendHeartbeat => {
                self.send_message(Heartbeat {}).await;
            }
            SessionMessage::SendMessage(message) => {
                self.send_message(message).await;
            }
            SessionMessage::Disconnected(reason) => {
                warn!(reason, "disconnected from peer");
                self.on_disconnect(reason).await;
            }
            SessionMessage::RegisterWriter(w) => {
                self.writer = Some(w);
                self.send_logon().await;
            }
            SessionMessage::ShouldReconnect(responder) => {
                responder
                    .send(self.state.should_reconnect())
                    .expect("be able to respond");
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
