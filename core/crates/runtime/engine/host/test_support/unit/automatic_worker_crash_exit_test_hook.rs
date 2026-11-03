use anyhow::Result;
use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};
use uuid::Uuid;

const AUTOMATIC_EXIT_HOOK_OCCUPIED: &str = "automatic crash exit hook is occupied";

type Hook = Box<dyn FnOnce() -> Result<()> + Send>;

fn hooks() -> &'static Mutex<HashMap<Uuid, Hook>> {
    static HOOKS: OnceLock<Mutex<HashMap<Uuid, Hook>>> = OnceLock::new();
    HOOKS.get_or_init(|| Mutex::new(HashMap::new()))
}

pub(in crate::host) struct Guard {
    terminal_id: Uuid,
}

pub(in crate::host) fn install(
    terminal_id: Uuid,
    hook: impl FnOnce() -> Result<()> + Send + 'static,
) -> Guard {
    let previous = super::lock_recover(hooks()).insert(terminal_id, Box::new(hook));
    assert!(previous.is_none(), "{AUTOMATIC_EXIT_HOOK_OCCUPIED}");
    Guard { terminal_id }
}

pub(in crate::host) fn run(terminal_id: Uuid) -> Result<()> {
    match super::lock_recover(hooks()).remove(&terminal_id) {
        Some(hook) => hook(),
        None => Ok(()),
    }
}

impl Drop for Guard {
    fn drop(&mut self) {
        super::lock_recover(hooks()).remove(&self.terminal_id);
    }
}
