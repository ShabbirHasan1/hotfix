use fefix::tagvalue::{Config, Encoder};
use fefix::TagU16;

pub fn create_login_message(sender_comp_id: &str, target_comp_id: &str) -> Vec<u8> {
    let mut buffer = Vec::new();
    let mut encoder: Encoder<Config> = Encoder::default();
    let mut msg = encoder.start_message(b"FIX.4.4", &mut buffer, b"A");
    msg.set_any(create_tag(49), sender_comp_id.as_bytes()); // sender comp id
    msg.set_any(create_tag(56), target_comp_id.as_bytes()); // target comp id
    msg.set_any(create_tag(34), b"1"); // msg sequence number
    msg.set_any(create_tag(52), b"20230912-08:24:56.574"); // sending time
    msg.set_any(create_tag(98), b"0"); // encrypt method
    msg.set_any(create_tag(108), b"30"); // heartbeat interval
    msg.set_any(create_tag(141), b"Y"); // reset seq num flag

    msg.wrap().to_vec()
}

fn create_tag(t: u16) -> TagU16 {
    TagU16::new(t).unwrap()
}
