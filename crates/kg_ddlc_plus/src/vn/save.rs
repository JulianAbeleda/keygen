use super::{ScreenCommand, ScreenResult};
#[derive(Clone, Debug)]
pub struct SaveScreen {
    pub slots: Vec<Option<String>>,
    pub selected: usize,
}
impl SaveScreen {
    pub fn new(count: usize) -> Self {
        Self {
            slots: vec![None; count],
            selected: 0,
        }
    }
    pub fn select(&mut self, n: usize) {
        if n < self.slots.len() {
            self.selected = n
        }
    }
    pub fn save(&self) -> ScreenResult {
        ScreenResult::consumed(Some(ScreenCommand::Save(self.selected)))
    }
    pub fn load(&self) -> ScreenResult {
        ScreenResult::consumed(Some(ScreenCommand::Load(self.selected)))
    }
}
