use crate::config::{Config, SessionConfig};
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

    pub fn get_number_of_sessions(&self) -> usize {
        self.sessions.len()
    }
}
