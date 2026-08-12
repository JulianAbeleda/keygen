use super::{ScreenCommand, ScreenResult, ScreenText};
#[derive(Clone, Debug, Default)]
pub struct HistoryScreen {
    pub lines: Vec<ScreenText>,
    pub offset: usize,
}
impl HistoryScreen {
    pub fn push(&mut self, line: ScreenText) {
        self.lines.push(line)
    }
    pub fn back(&self) -> ScreenResult {
        ScreenResult::consumed(Some(ScreenCommand::Back))
    }
    pub fn scroll(&mut self, d: isize) {
        self.offset = (self.offset as isize + d).max(0) as usize;
        self.offset = self.offset.min(self.lines.len().saturating_sub(1));
    }
}
