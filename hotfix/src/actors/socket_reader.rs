use crate::actors::orchestrator::OrchestratorHandle;
use crate::message::parser::Parser;
use tokio::io::{AsyncReadExt, ReadHalf};

use crate::tls_client::FixStream;

pub async fn reader_loop(mut reader: ReadHalf<FixStream>, orchestrator: OrchestratorHandle) {
    let mut parser = Parser::default();
    loop {
        let mut buf = vec![];
        reader.read_buf(&mut buf).await.unwrap();
        let messages = parser.parse(&buf);

        for msg in messages {
            orchestrator.new_fix_message_received(msg).await;
        }
    }
}
