pub mod in_memory;

#[async_trait::async_trait]
pub trait MessageStore {
    async fn add(&mut self, sequence_number: u64, message: Vec<u8>);
    async fn next_sender_seq_number(&self) -> u64;
    async fn next_target_seq_number(&self) -> u64;
    async fn increment_sender_seq_number(&mut self);
    async fn increment_target_seq_number(&mut self);
}
