use super::CodexBindingAuthority;
use anyhow::Result;

impl CodexBindingAuthority {
    pub fn unavailable_for_test(&self) -> Result<Self> {
        self.unavailable()
    }

    pub fn revoked_for_test(&self) -> Result<Self> {
        self.revoked()
    }
}
