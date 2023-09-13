use crate::config::SessionConfig;
use crate::message::logon;
use crate::tls_client::{Client, FixStream};
use tokio::io::{AsyncReadExt, AsyncWriteExt, ReadHalf, WriteHalf};
use tokio::select;
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};
use tracing::{debug, info};

pub struct Message {
    is_logon: bool,
}

pub struct Session {
    pub config: SessionConfig,
    pub sender: UnboundedSender<Message>,
}

impl Session {
    pub fn new(config: SessionConfig) -> Self {
        let (tx, rx) = tokio::sync::mpsc::unbounded_channel();

        let spawned_config = config.clone();
        tokio::spawn(async move {
            establish_connection(spawned_config, rx).await;
        });
        tx.send(Message { is_logon: true }).unwrap();
        Self { config, sender: tx }
    }

    pub async fn send_message(&self, msg: Message) {
        self.sender.send(msg).unwrap();
    }
}

async fn establish_connection(config: SessionConfig, recv: UnboundedReceiver<Message>) {
    let tls_client = Client::new(&config).await;
    let (reader, writer) = tls_client.split();

    let fut_writer = writer_loop(config, writer, recv);
    let fut_reader = reader_loop(reader);

    select! {
        () = fut_writer => {
            info!("writer loop closed")
        }
        () = fut_reader => {
            info!("reader loop closed")
        }
    }
}

async fn writer_loop(
    config: SessionConfig,
    mut writer: WriteHalf<FixStream>,
    mut message_channel: UnboundedReceiver<Message>,
) {
    while let Some(msg) = message_channel.recv().await {
        if msg.is_logon {
            let login_message =
                logon::create_login_message(&config.sender_comp_id, &config.target_comp_id);
            writer
                .write_all(&login_message)
                .await
                .expect("logon message to send");
            debug!("sent logon message");
        } else {
            debug!("received non-logon message");
        }
    }
    debug!("writer received None, closing the task");
}

async fn reader_loop(mut reader: ReadHalf<FixStream>) {
    loop {
        let mut buf = vec![];
        reader.read_buf(&mut buf).await.unwrap();

        let msg = String::from_utf8(buf).unwrap();
        debug!("received message: {}", msg);
    }
}
