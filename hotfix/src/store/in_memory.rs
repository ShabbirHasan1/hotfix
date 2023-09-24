use std::collections::HashMap;

use crate::store::MessageStore;

#[derive(Debug, Default)]
pub struct InMemoryMessageStore {
    sender_seq_number: u64,
    target_seq_number: u64,
    messages: HashMap<u64, Vec<u8>>,
}

#[async_trait::async_trait]
impl MessageStore for InMemoryMessageStore {
    async fn add(&mut self, sequence_number: u64, message: &[u8]) {
        self.messages.insert(sequence_number, message.to_vec());
    }

    async fn next_sender_seq_number(&self) -> u64 {
        self.sender_seq_number + 1
    }

    async fn next_target_seq_number(&self) -> u64 {
        self.target_seq_number + 1
    }

    async fn increment_sender_seq_number(&mut self) {
        self.sender_seq_number += 1;
    }

    async fn increment_target_seq_number(&mut self) {
        self.target_seq_number += 1;
    }
}
