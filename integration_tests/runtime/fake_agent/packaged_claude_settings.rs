pub(crate) use crate::provider_hooks::{HookCommand, Settings};
use std::error::Error;
use std::path::Path;

const CONFIGURATION_FILE: &str = "settings.json";

pub(crate) fn load(home: &Path) -> Result<Settings, Box<dyn Error>> {
    crate::provider_hooks::load(&home.join(CONFIGURATION_FILE))
}
