use fefix::definitions::fix44;
use fefix::fix_values::Timestamp;
use fefix::tagvalue::{Config, Encoder, FvWrite};

pub fn logon_message(sender_comp_id: &str, target_comp_id: &str, msg_seq_num: usize) -> Vec<u8> {
    let mut buffer = Vec::new();
    let mut encoder: Encoder<Config> = Encoder::default();
    let mut msg = encoder.start_message(b"FIX.4.4", &mut buffer, b"A");
    msg.set_fv(fix44::SENDER_COMP_ID, sender_comp_id);
    msg.set_fv(fix44::TARGET_COMP_ID, target_comp_id);
    msg.set_fv(fix44::MSG_SEQ_NUM, msg_seq_num);
    msg.set_fv(fix44::SENDING_TIME, Timestamp::utc_now());
    msg.set_fv(fix44::ENCRYPT_METHOD, fix44::EncryptMethod::None);
    msg.set_fv(fix44::HEART_BT_INT, 30);
    msg.set_fv(fix44::RESET_SEQ_NUM_FLAG, fix44::ResetSeqNumFlag::Yes);

    msg.wrap().to_vec()
}
