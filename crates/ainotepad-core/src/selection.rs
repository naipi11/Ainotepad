pub type Offset = usize;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Selection {
    pub anchor: Offset,
    pub caret: Offset,
}

impl Selection {
    pub fn is_empty(self) -> bool {
        self.anchor == self.caret
    }

    pub fn start(self) -> Offset {
        self.anchor.min(self.caret)
    }

    pub fn end(self) -> Offset {
        self.anchor.max(self.caret)
    }
}
