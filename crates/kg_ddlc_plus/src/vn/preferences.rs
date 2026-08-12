use super::{ScreenCommand, ScreenResult};
use keygen_player::storage::Preferences;
#[derive(Clone, Debug)]
pub struct PreferencesScreen {
    pub value: Preferences,
}
impl PreferencesScreen {
    pub fn new(value: Preferences) -> Result<Self, String> {
        value.validate()?;
        Ok(Self { value })
    }
    pub fn apply(&self) -> ScreenResult {
        ScreenResult::consumed(Some(ScreenCommand::SetPreference(self.value.clone())))
    }
}
