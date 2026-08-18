use egui::{Event, ImeEvent};

#[derive(Clone, Debug, Default)]
pub struct ImeState {
    pub composing: bool,
    pub preedit: String,
}

pub enum ImeAction {
    None,
    PreeditChanged,
    Commit(String),
}

impl ImeState {
    pub fn on_event(&mut self, event: &Event) -> ImeAction {
        let Event::Ime(ime) = event else {
            return ImeAction::None;
        };
        match ime {
            ImeEvent::Enabled => {
                self.composing = true;
                ImeAction::None
            }
            ImeEvent::Preedit(text) => {
                self.composing = true;
                self.preedit = text.clone();
                ImeAction::PreeditChanged
            }
            ImeEvent::Commit(text) => {
                self.composing = false;
                self.preedit.clear();
                ImeAction::Commit(text.clone())
            }
            ImeEvent::Disabled => {
                self.composing = false;
                self.preedit.clear();
                ImeAction::None
            }
        }
    }
}
