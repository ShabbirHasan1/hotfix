use fefix::TagU16;

pub fn create_tag(t: u16) -> TagU16 {
    TagU16::new(t).unwrap()
}
