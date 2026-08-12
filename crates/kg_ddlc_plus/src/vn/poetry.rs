use super::{ScreenCommand, ScreenResult};
#[derive(Clone, Debug)]
pub struct PoetryScreen {
    pub words: Vec<String>,
    pub selected: Option<usize>,
    pub submitted: bool,
}
impl PoetryScreen {
    pub fn new(words: Vec<String>) -> Result<Self, String> {
        if words.is_empty() {
            Err("poetry screen needs words".into())
        } else {
            Ok(Self {
                words,
                selected: None,
                submitted: false,
            })
        }
    }
    pub fn choose(&mut self, n: usize) {
        if n < self.words.len() {
            self.selected = Some(n)
        }
    }
    pub fn submit(&mut self) -> ScreenResult {
        self.submitted = true;
        ScreenResult::consumed(Some(ScreenCommand::Advance))
    }
}
