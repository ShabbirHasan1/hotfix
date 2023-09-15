use tokio::select;
use tokio::sync::mpsc;
use tokio::time::{sleep, Duration, Instant};
use tracing::debug;

use crate::actors::socket_writer::WriterHandle;
use crate::builtin_messages::generate_message;
use crate::config::SessionConfig;
use crate::message::hardcoded::FixMessage;
use crate::message::heartbeat::heartbeat_message;
use crate::message::logon::logon_message;
use crate::message::parser::RawFixMessage;

#[derive(Clone, Debug)]
pub enum OrchestratorMessage {
    FixMessageReceived(RawFixMessage),
    SendHeartbeat,
    SendLogon,
    SendMessage(FixMessage),
}

#[derive(Clone)]
pub struct OrchestratorHandle {
    sender: mpsc::Sender<OrchestratorMessage>,
}

impl OrchestratorHandle {
    pub fn new(config: SessionConfig, writer: WriterHandle) -> Self {
        let (sender, mailbox) = mpsc::channel(10);
        let actor = OrchestratorActor::new(mailbox, config, writer);
        tokio::spawn(run_orchestrator(actor));

        Self { sender }
    }

    pub async fn new_fix_message_received(&self, msg: RawFixMessage) {
        self.sender
            .send(OrchestratorMessage::FixMessageReceived(msg))
            .await
            .expect("be able to receive message");
    }

    pub async fn send_message(&self, msg: FixMessage) {
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

struct OrchestratorActor {
    mailbox: mpsc::Receiver<OrchestratorMessage>,
    config: SessionConfig,
    writer: WriterHandle,
    msg_seq_number: usize,
}

impl OrchestratorActor {
    fn new(
        mailbox: mpsc::Receiver<OrchestratorMessage>,
        config: SessionConfig,
        writer: WriterHandle,
    ) -> OrchestratorActor {
        Self {
            mailbox,
            config,
            writer,
            msg_seq_number: 0,
        }
    }

    fn next_sequence_number(&mut self) -> usize {
        self.msg_seq_number += 1;
        self.msg_seq_number
    }

    async fn handle(&mut self, message: OrchestratorMessage) -> HandleOutput {
        match message {
            OrchestratorMessage::FixMessageReceived(fix_message) => {
                debug!("received message: {}", fix_message);
            }
            OrchestratorMessage::SendHeartbeat => {
                let seq_num = self.next_sequence_number();
                let msg = heartbeat_message(
                    &self.config.sender_comp_id,
                    &self.config.target_comp_id,
                    seq_num,
                );
                self.writer.send_raw_message(RawFixMessage::new(msg)).await;
                return HandleOutput::new(true);
            }
            OrchestratorMessage::SendLogon => {
                let seq_num = self.next_sequence_number();
                let msg = logon_message(
                    &self.config.sender_comp_id,
                    &self.config.target_comp_id,
                    seq_num,
                );
                self.writer.send_raw_message(RawFixMessage::new(msg)).await;
                return HandleOutput::new(true);
            }
            OrchestratorMessage::SendMessage(msg) => {
                let seq_num = self.next_sequence_number();
                let raw_message = generate_message(
                    &self.config.sender_comp_id,
                    &self.config.target_comp_id,
                    seq_num,
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

async fn run_orchestrator(mut actor: OrchestratorActor) {
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
