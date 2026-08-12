use super::{ScreenCommand, ScreenResult};
#[derive(Clone, Debug)]
pub struct MainMenu {
    pub entries: Vec<String>,
    pub selected: usize,
}
impl MainMenu {
    pub fn new(entries: Vec<String>) -> Result<Self, String> {
        if entries.is_empty() {
            return Err("menu needs entries".into());
        }
        Ok(Self {
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
