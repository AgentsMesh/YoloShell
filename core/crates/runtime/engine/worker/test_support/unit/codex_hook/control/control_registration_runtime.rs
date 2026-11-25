use super::CodexHookRuntime;

impl CodexHookRuntime {
    pub(in crate::worker::codex_hook) fn retire_control_registration_for_test(&self) {
        self.registration.retire_control_registration_for_test();
    }
}
