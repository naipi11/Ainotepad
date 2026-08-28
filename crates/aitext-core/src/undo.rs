use crate::selection::{Offset, Selection};

#[derive(Clone, Debug)]
pub struct Edit {
    pub delete_start: Offset,
    pub deleted: String,
    pub insert_start: Offset,
    pub inserted: String,
    pub before: Selection,
    pub after: Selection,
    pub coalesce_inserts: bool,
}
