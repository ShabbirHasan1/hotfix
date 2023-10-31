use std::fmt::Write;

use super::{ERR_INT_INVALID, ERR_UTF8};
use crate::{Buffer, BufferWriter, FieldType};
use hotfix_dictionary::TagU32;

impl<'a> FieldType<'a> for TagU32 {
    type Error = &'static str;
    type SerializeSettings = ();

    #[inline]
    fn serialize_with<B>(&self, buffer: &mut B, _settings: ()) -> usize
    where
        B: Buffer,
    {
        let initial_len = buffer.len();
        write!(BufferWriter(buffer), "{}", self).unwrap();
        buffer.len() - initial_len
    }

    #[inline]
    fn deserialize(data: &'a [u8]) -> Result<Self, Self::Error> {
        std::str::from_utf8(data)
            .map_err(|_| ERR_UTF8)?
            .parse()
            .map_err(|_| ERR_INT_INVALID)
    }

    #[inline]
    fn deserialize_lossy(data: &'a [u8]) -> Result<Self, Self::Error> {
        let n = u32::deserialize_lossy(data)?;
        Ok(TagU32::new(n.max(1)).unwrap())
    }
}
