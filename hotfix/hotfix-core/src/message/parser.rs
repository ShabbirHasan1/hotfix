//! HeaderInfo taken from ferrumfix
use std::ops::Range;

#[derive(Debug, Clone)]
pub struct HeaderInfo {
    field_0: Range<usize>,
    field_1: Range<usize>,
    nominal_body_len: usize,
}

impl HeaderInfo {
    fn parse(data: &[u8], separator: u8) -> Option<Self> {
        let mut info = Self {
            field_0: 0..1,
            field_1: 0..1,
            nominal_body_len: 0,
        };

        let mut iterator = data.iter();
        let mut find_byte = |byte| iterator.position(|b| *b == byte);
        let mut i = 0;

        i += find_byte(b'=')? + 1;
        info.field_0.start = i;
        i += find_byte(separator)?;
        info.field_0.end = i;
        i += 1;

        i += find_byte(b'=')? + 1;
        info.field_1.start = i;
        i += find_byte(separator)?;
        info.field_1.end = i;

        for byte in &data[info.field_1.clone()] {
            info.nominal_body_len = info
                .nominal_body_len
                .wrapping_mul(10)
                .wrapping_add(byte.wrapping_sub(b'0') as usize);
        }

        Some(info)
    }

    #[inline]
    fn nominal_body_length(&self) -> usize {
        self.nominal_body_len
    }
}

#[cfg(test)]
mod tests {
    use crate::message::parser::HeaderInfo;

    #[test]
    fn test_incomplete_header_info() {
        let data = b"8=FIX.4.4\x019";
        let result = HeaderInfo::parse(data, b'\x01');

        assert!(result.is_none());
    }

    #[test]
    fn test_exact_header() {
        let data = b"8=FIX.4.4\x019=77\x01";
        let result = HeaderInfo::parse(data, b'\x01');

        assert!(result.is_some());
        assert_eq!(result.unwrap().nominal_body_length(), 77);
    }

    #[test]
    fn test_full_message() {
        let data = b"8=FIX.4.4\x019=77\x0135=A\x0134=1\x0149=validus-fix\x0152=20230908-08:24:56.574\x0156=FXALL\x0198=0\x01108=30\x01141=Y\x0110=037\x01";
        let result = HeaderInfo::parse(data, b'\x01');

        assert!(result.is_some());
        assert_eq!(result.unwrap().nominal_body_length(), 77);
    }
}
