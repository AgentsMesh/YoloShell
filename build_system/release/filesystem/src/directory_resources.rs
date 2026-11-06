use anyhow::{Context, Result, ensure};
use std::sync::{Arc, Mutex};

const TRACKER_POISONED: &str = "release directory resource custody is indeterminate";
const RETIRED: &str = "release directory resource owner has retired";
const UNSETTLED: &str = "release directory resources remain unsettled";

#[derive(Clone, Debug, Default)]
pub(crate) struct DirectoryResources(Arc<Mutex<State>>);

#[derive(Debug, Default)]
struct State {
    active: usize,
    uncertain: Vec<String>,
    retired: bool,
}

#[must_use = "directory use requires positive release evidence"]
#[derive(Debug)]
pub struct DirectoryResourceUse {
    guards: Vec<ResourceGuard>,
}

#[derive(Debug)]
struct ResourceGuard {
    resources: DirectoryResources,
    description: String,
    confirmed: bool,
}

#[derive(Debug)]
pub struct DirectoryResourceOwner(DirectoryResources);

impl DirectoryResourceOwner {
    /// Attach before sharing this handle; descendants also retain ancestor owners.
    pub fn attach(directory: &mut crate::DirectoryHandle) -> Self {
        let resources = DirectoryResources::default();
        directory.resources.push(resources.clone());
        Self(resources)
    }

    pub fn begin_retirement(&self) -> Result<()> {
        self.0.retire()
    }
}

impl DirectoryResources {
    fn retain(&self, description: &str) -> Result<ResourceGuard> {
        let mut state = self
            .0
            .lock()
            .map_err(|_| anyhow::anyhow!(TRACKER_POISONED))?;
        ensure!(!state.retired, RETIRED);
        state.active = state.active.checked_add(1).context(UNSETTLED)?;
        Ok(ResourceGuard {
            resources: self.clone(),
            description: description.to_owned(),
            confirmed: false,
        })
    }

    pub(crate) fn retire(&self) -> Result<()> {
        let mut state = self
            .0
            .lock()
            .map_err(|_| anyhow::anyhow!(TRACKER_POISONED))?;
        ensure!(
            state.active == 0 && state.uncertain.is_empty(),
            "{UNSETTLED}: {} active; {}",
            state.active,
            state.uncertain.join("; ")
        );
        state.retired = true;
        Ok(())
    }
}

impl crate::DirectoryHandle {
    pub fn retain_resource_use(&self, description: &str) -> Result<DirectoryResourceUse> {
        DirectoryResourceUse::retain([self], description)
    }
}

impl DirectoryResourceUse {
    pub fn retain<'a>(
        directories: impl IntoIterator<Item = &'a crate::DirectoryHandle>,
        description: &str,
    ) -> Result<Self> {
        let mut usage = DirectoryResourceUse { guards: Vec::new() };
        for resources in directories
            .into_iter()
            .flat_map(|directory| &directory.resources)
        {
            match resources.retain(description) {
                Ok(guard) => usage.guards.push(guard),
                Err(error) => {
                    usage.confirm_released();
                    return Err(error);
                }
            }
        }
        Ok(usage)
    }
    /// Only call after proving that the external consumer cannot use the directory.
    pub fn confirm_released(mut self) {
        for guard in &mut self.guards {
            guard.confirmed = true;
        }
    }
}

impl Drop for ResourceGuard {
    fn drop(&mut self) {
        // A poisoned tracker remains permanently non-retirable.
        if let Ok(mut state) = self.resources.0.lock() {
            state.active -= 1;
            if !self.confirmed {
                state.uncertain.push(self.description.clone());
            }
        }
    }
}

#[cfg(all(test, feature = "yolo-unit-tests"))]
#[path = "../unit_tests/resource_state.rs"]
mod tests;
