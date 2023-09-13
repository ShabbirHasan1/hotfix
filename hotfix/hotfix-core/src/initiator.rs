use crate::config::{Config, SessionConfig};
use crate::session::Session;

pub struct SocketInitiator {
    sessions: Vec<Session>,
}

impl SocketInitiator {
    pub async fn new(config: Config) -> Self {
        let sessions = config
            .sessions
            .into_iter()
            .map(Self::create_session)
            .collect();
        Self { sessions }
    }

    fn create_session(config: SessionConfig) -> Session {
        Session::new(config)
    }

    pub fn get_number_of_sessions(&self) -> usize {
        self.sessions.len()
    }
}
