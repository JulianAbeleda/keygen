use super::{ScreenCommand, ScreenResult};
#[derive(Clone, Debug)]
pub struct SpecialScreen {
    pub id: String,
    pub acknowledged: bool,
}
impl SpecialScreen {
    pub fn new(id: impl Into<String>) -> Result<Self, String> {
        let id = id.into();
        if id.trim().is_empty() {
            Err("special screen id is empty".into())
        } else {
            Ok(Self {
                id,
                acknowledged: false,
            })
        }
    }
    pub fn acknowledge(&mut self) -> ScreenResult {
        self.acknowledged = true;
        ScreenResult::consumed(Some(ScreenCommand::Advance))
    }
}
