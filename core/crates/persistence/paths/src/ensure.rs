use super::*;

impl Paths {
    pub fn ensure_base(&self) -> Result<()> {
        validate_home(&self.home)?;
        crate::paths::directory::PrivateDirectory::open_or_create_strict(&self.home)?;
        create_private_dir(&self.runtime_dir())?;
        create_private_dir(&self.sessions_dir())
    }

    pub fn ensure_session(&self, id: Uuid) -> Result<()> {
        self.ensure_base()?;
        create_private_dir(&self.session_dir(id))?;
        create_private_dir(&self.terminal_dir(id))?;
        create_private_dir(&self.events_dir(id))?;
        create_private_dir(&self.session_dir(id).join(LOG_DIRECTORY))
    }
}
