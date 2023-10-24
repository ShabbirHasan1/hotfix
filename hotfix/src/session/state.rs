use crate::actors::socket_writer::WriterRef;
use crate::message::parser::RawFixMessage;
use tracing::{debug, error};

pub enum SessionState {
    Connected { writer: WriterRef },
    LoggedOut { reconnect: bool },
    Disconnected { reconnect: bool, reason: String },
}

impl SessionState {
    pub fn should_reconnect(&self) -> bool {
        match self {
            SessionState::Disconnected { reconnect, .. } => *reconnect,
            _ => true,
        }
    }

    pub async fn send_message(&self, message: RawFixMessage) {
        match self {
            Self::Connected { writer } => writer.send_raw_message(message).await,
            _ => error!("trying to write without an established connection"),
        }
    }

    pub async fn disconnect(&self) {
        match self {
            Self::Connected { writer } => writer.disconnect().await,
            _ => debug!("disconnecting an already disconnected session"),
        }
    }
}
