use crate::config::{Config, SessionConfig};
use crate::message::hardcoded::FixMessage;
use crate::session::Session;
use futures::future::join_all;

pub struct SocketInitiator {
    sessions: Vec<Session>,
}

impl SocketInitiator {
    pub async fn new(config: Config) -> Self {
        let fut_sessions: Vec<_> = config
            .sessions
            .into_iter()
            .map(Self::create_session)
            .collect();
        let sessions = join_all(fut_sessions).await;
        Self { sessions }
    }

    async fn create_session(config: SessionConfig) -> Session {
        Session::new(config).await
    }

    pub async fn send_message(&self, sender_comp_id: &str, target_comp_id: &str, msg: FixMessage) {
        let fut: Vec<_> = self
            .sessions
            .iter()
            .filter(|s| s.is_interested(sender_comp_id, target_comp_id))
            .map(|s| s.send_message(msg.clone()))
            .collect();
        join_all(fut).await;
    }

    pub fn get_number_of_sessions(&self) -> usize {
        self.sessions.len()
    }
}
