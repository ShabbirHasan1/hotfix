//! Crate implementing the encoding (presentation) layer.
mod buffer;
pub mod config;
pub mod decoder;
mod definitions;
mod encoder;
mod error;
pub mod field_access;
pub mod field_types;
pub mod raw_decoder;
mod streaming_decoder;
mod utils;

pub use hotfix_derive::FieldType;
pub use hotfix_dictionary::{self as dict, TagU32};

use buffer::{Buffer, BufferWriter};
pub use config::{Config, GetConfig};
pub use decoder::{Decoder, Message};
pub use definitions::fix44;
pub use encoder::{Encoder, EncoderHandle, SetField};
use field_access::FieldType;
