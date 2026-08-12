use super::{ScreenCommand, ScreenResult};
#[derive(Clone, Debug)]
pub struct ChoiceScreen {
    pub prompt: String,
    pub entries: Vec<String>,
    pub selected: usize,
}
impl ChoiceScreen {
    pub fn new(prompt: impl Into<String>, entries: Vec<String>) -> Result<Self, String> {
        if entries.is_empty() {
            return Err("choice needs entries".into());
        }
        Ok(Self {
            prompt: prompt.into(),
            entries,
            selected: 0,
        })
    }
    pub fn move_by(&mut self, d: isize) {
        self.selected =
            (self.selected as isize + d).rem_euclid(self.entries.len() as isize) as usize
    }
    pub fn confirm(&self) -> ScreenResult {
        ScreenResult::consumed(Some(ScreenCommand::Select(self.selected)))
    }
}
