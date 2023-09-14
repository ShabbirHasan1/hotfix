use fefix::fix_values::Timestamp;
use fefix::tagvalue::{Config, Encoder};

use crate::message::common::create_tag;

pub fn logon_message(sender_comp_id: &str, target_comp_id: &str, msg_seq_num: usize) -> Vec<u8> {
    let mut buffer = Vec::new();
    let mut encoder: Encoder<Config> = Encoder::default();
    let mut msg = encoder.start_message(b"FIX.4.4", &mut buffer, b"A");
    msg.set_any(create_tag(49), sender_comp_id.as_bytes()); // sender comp id
    msg.set_any(create_tag(56), target_comp_id.as_bytes()); // target comp id
    msg.set_any(create_tag(34), msg_seq_num); // msg sequence number
    msg.set_any(create_tag(52), Timestamp::utc_now()); // sending time
    msg.set_any(create_tag(98), b"0"); // encrypt method
    msg.set_any(create_tag(108), b"30"); // heartbeat interval
    msg.set_any(create_tag(141), b"Y"); // reset seq num flag

    msg.wrap().to_vec()
}
