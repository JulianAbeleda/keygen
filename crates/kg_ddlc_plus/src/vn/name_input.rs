use super::{ScreenCommand, ScreenResult};
#[derive(Clone, Debug)]
pub struct NameInput {
    pub value: String,
    pub max_chars: usize,
}
impl NameInput {
    pub fn new(max_chars: usize) -> Result<Self, String> {
        if max_chars == 0 {
            Err("name limit must be positive".into())
        } else {
            Ok(Self {
                value: String::new(),
                max_chars,
            })
        }
    }
    pub fn insert(&mut self, text: &str) {
        for c in text.chars() {
            if self.value.chars().count() < self.max_chars {
                self.value.push(c)
            }
        }
    }
    pub fn confirm(&self) -> Result<ScreenResult, String> {
        if self.value.trim().is_empty() {
            Err("name cannot be empty".into())
        } else {
            Ok(ScreenResult::consumed(Some(ScreenCommand::SetName(
                self.value.clone(),
            ))))
        }
    }
}
