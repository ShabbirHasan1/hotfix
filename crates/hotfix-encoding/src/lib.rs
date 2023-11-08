//! Crate implementing the encoding (presentation) layer.
mod buffer;
mod definitions;
pub mod field_access;
pub mod field_types;

pub use hotfix_derive::FieldType;
pub use hotfix_dictionary::{self as dict, TagU32};

use buffer::{Buffer, BufferWriter};
#[cfg(feature = "fix42")]
pub use definitions::fix42;
pub use definitions::{fix44, HardCodedFixFieldDefinition};
use field_access::FieldType;
