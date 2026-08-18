pub mod document;
pub mod encoding;
pub mod motion;
pub mod selection;
pub mod undo;

pub use document::Document;
pub use encoding::{
    classify_size, decode_bytes, encode_text, majority_newline, DecodeError, EncodeError, Encoding,
    NewlineStyle, OpenError, SizeClass, HARD_LIMIT_BYTES, SOFT_LIMIT_BYTES,
};
pub use motion::{Motion, PAGE_LINES};
pub use selection::{Offset, Selection};
