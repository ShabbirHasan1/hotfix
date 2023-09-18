use fefix::definitions::fix44;
use fefix::tagvalue::{EncoderHandle, FvWrite};

use crate::message::{DecodedMessage, FixMessage};

#[derive(Clone, Debug)]
pub struct Logon {
    encrypt_method: fix44::EncryptMethod,
    heartbeat_interval: u64,
    reset_seq_num_flag: fix44::ResetSeqNumFlag,
}

impl Logon {
    pub fn new(heartbeat_interval: u64) -> Self {
        Self {
            encrypt_method: fix44::EncryptMethod::None,
            heartbeat_interval,
            reset_seq_num_flag: fix44::ResetSeqNumFlag::Yes,
        }
    }
}

impl FixMessage for Logon {
    fn write(&self, msg: &mut EncoderHandle<Vec<u8>>) {
        msg.set_fv(fix44::ENCRYPT_METHOD, self.encrypt_method);
        msg.set_fv(fix44::HEART_BT_INT, self.heartbeat_interval);
        msg.set_fv(fix44::RESET_SEQ_NUM_FLAG, self.reset_seq_num_flag);
    }

    fn message_type(&self) -> &[u8] {
        b"A"
    }

    fn parse(_message: DecodedMessage<&[u8]>) -> Self {
        todo!()
    }
}
