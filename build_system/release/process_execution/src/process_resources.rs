use crate::ProcessCommand;
use anyhow::Result;
use yolo_release_filesystem::DirectoryResourceUse;

#[derive(Default)]
pub(crate) struct ProcessResources {
    usage: Option<DirectoryResourceUse>,
    spawned: bool,
}

impl ProcessResources {
    pub(crate) fn acquire(command: &ProcessCommand) -> Result<Self> {
        let directories = [
            &command.workspace,
            &command.home,
            &command.temporary_directory,
        ]
        .into_iter()
        .chain(command.resource_directories.iter());
        Ok(Self {
            usage: Some(DirectoryResourceUse::retain(
                directories,
                &command.description,
            )?),
            spawned: false,
        })
    }

    pub(crate) fn mark_spawned(&mut self) {
        self.spawned = true;
    }

    pub(crate) fn confirm_released(&mut self) {
        if let Some(usage) = self.usage.take() {
            usage.confirm_released();
        }
    }
}

impl Drop for ProcessResources {
    fn drop(&mut self) {
        if !self.spawned {
            self.confirm_released();
        }
        // A spawned process without positive settlement poisons each resource owner.
    }
}
