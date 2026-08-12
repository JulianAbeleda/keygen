use super::{typewriter::Typewriter, ScreenCommand, ScreenResult, ScreenText};
#[derive(Clone, Debug)]
pub struct DialogueScreen {
    pub line: Typewriter,
    pub auto: bool,
}
impl DialogueScreen {
    pub fn new(text: ScreenText, cps: f32) -> Result<Self, String> {
        Ok(Self {
            line: Typewriter::new(text, cps)?,
            auto: false,
        })
    }
    pub fn confirm(&mut self) -> ScreenResult {
        self.line.activate()
    }
    pub fn command(&self) -> Option<ScreenCommand> {
        self.auto.then_some(ScreenCommand::Advance)
    }
}
