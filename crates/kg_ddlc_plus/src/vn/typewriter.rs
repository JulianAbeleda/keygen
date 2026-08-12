use super::{ScreenCommand, ScreenResult, ScreenText};
#[derive(Clone, Debug, PartialEq)]
pub struct Typewriter {
    pub text: ScreenText,
    pub revealed: usize,
    pub cps: f32,
    pub complete: bool,
}
impl Typewriter {
    pub fn new(text: ScreenText, cps: f32) -> Result<Self, String> {
        if !cps.is_finite() || cps <= 0.0 {
            return Err("typewriter speed must be positive".into());
        }
        Ok(Self {
            text,
            revealed: 0,
            cps,
            complete: false,
        })
    }
    pub fn tick(&mut self, seconds: f32) {
        self.revealed = (seconds.max(0.0) * self.cps).floor() as usize;
        self.revealed = self.revealed.min(self.text.text.chars().count());
        self.complete = self.revealed == self.text.text.chars().count()
    }
    pub fn visible(&self) -> String {
        self.text.text.chars().take(self.revealed).collect()
    }
    pub fn activate(&mut self) -> ScreenResult {
        if self.complete {
            ScreenResult::consumed(Some(ScreenCommand::Advance))
        } else {
            self.revealed = self.text.text.chars().count();
            self.complete = true;
            ScreenResult::consumed(None)
        }
    }
}
