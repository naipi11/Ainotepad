pub mod document;
pub mod motion;
pub mod selection;
pub mod undo;

pub use document::Document;
pub use motion::{Motion, PAGE_LINES};
pub use selection::{Offset, Selection};
