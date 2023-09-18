use futures::future::join_all;

use crate::config::{Config, SessionConfig};
use crate::message::FixMessage;
use crate::session::Session;

pub struct SocketInitiator<M> {
    sessions: Vec<Session<M>>,
}

impl<M: FixMessage> SocketInitiator<M> {
    pub async fn new(config: Config) -> Self {
        let fut_sessions: Vec<_> = config
            .sessions
            .into_iter()
            .map(Self::create_session)
            .collect();
        let sessions = join_all(fut_sessions).await;
        Self { sessions }
    }

    async fn create_session(config: SessionConfig) -> Session<M> {
        Session::new(config).await
    }

    pub async fn send_message(&self, sender_comp_id: &str, target_comp_id: &str, msg: M) {
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
