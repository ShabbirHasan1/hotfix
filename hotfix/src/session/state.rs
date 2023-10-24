pub enum SessionState {
    Connected,
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
}
