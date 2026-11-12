pub(crate) const NO_SESSION: &str =
    "No matching terminal. Press c to create one or n to choose a directory.";
pub(crate) const AMBIGUOUS_SESSION: &str =
    "Several terminals match; attach using a terminal UUID or HUB/TERMINAL address.";
pub(crate) const RETAINED_SESSION: &str =
    "Saved terminal is unavailable. Press h for history, then R to resume.";
pub(crate) const ENDED_SESSION: &str =
    "Terminal is not running. Press R to resume it from history.";
pub(crate) const PENDING_CREATION: &str =
    "Checking an earlier shell creation; no additional shell was created.";
pub(crate) const STARTING_SHELL: &str = "Starting shell…";
pub(crate) const SEARCHING_TERMINAL: &str = "Looking for terminal…";
/// An explicit attach may precede the catalogue page that carries its target.
pub(crate) const STARTUP_SELECTION_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(5);
