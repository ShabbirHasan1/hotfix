use fefix::definitions::fix44;
use fefix::fix_values::Timestamp;
use fefix::tagvalue::{Config, Encoder, FvWrite};

pub fn heartbeat_message(
    sender_comp_id: &str,
    target_comp_id: &str,
    msg_seq_num: usize,
) -> Vec<u8> {
    let mut buffer = Vec::new();
    let mut encoder: Encoder<Config> = Encoder::default();
    let mut msg = encoder.start_message(b"FIX.4.4", &mut buffer, b"0");
    msg.set_fv(fix44::SENDER_COMP_ID, sender_comp_id);
    msg.set_fv(fix44::TARGET_COMP_ID, target_comp_id);
    msg.set_fv(fix44::MSG_SEQ_NUM, msg_seq_num);
    msg.set_fv(fix44::SENDING_TIME, Timestamp::utc_now());

    msg.wrap().to_vec()
}
