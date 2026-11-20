use super::CodexAdmissionAuthority;

impl CodexAdmissionAuthority {
    pub fn retirement_seal_path_for_test(&self) -> std::path::PathBuf {
        self.storage
            .directory()
            .path()
            .join(super::super::retirement_constants::SEAL_FILE_NAME)
    }
}
