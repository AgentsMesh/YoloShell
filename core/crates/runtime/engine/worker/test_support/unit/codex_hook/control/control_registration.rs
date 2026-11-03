use super::CodexAuthorityRegistration;

impl CodexAuthorityRegistration {
    pub(in crate::worker::codex_hook) fn retire_control_registration_for_test(&self) {
        let _takeover = self.slot.takeover.lock().unwrap();
        self.slot.remove_exact(self.token, &self.control).unwrap();
    }
}
