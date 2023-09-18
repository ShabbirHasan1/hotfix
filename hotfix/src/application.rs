use crate::message::FixMessage;

#[async_trait::async_trait]
trait Application {
    async fn on_message_from_app(msg: impl FixMessage);
    async fn on_message_to_app(msg: impl FixMessage);
}
