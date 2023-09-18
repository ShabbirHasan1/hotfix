use fefix::tagvalue::EncoderHandle;

use crate::message::FixMessage;

#[derive(Clone, Debug)]
pub struct Heartbeat;

impl FixMessage for Heartbeat {
    fn write(&self, _msg: &mut EncoderHandle<Vec<u8>>) {}

    fn message_type(&self) -> &[u8] {
        b"0"
    }
}
