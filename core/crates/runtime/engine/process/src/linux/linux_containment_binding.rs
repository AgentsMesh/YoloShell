use super::{LinuxContainmentError, LinuxContainmentLease};
use crate::process::{ProcessIdentity, PtySessionProof};

const PTY_ALREADY_BOUND_DETAIL: &str = "containment PTY proof is already bound";
const PTY_DEVICE_INVALID_DETAIL: &str = "containment PTY device is invalid";
const PTY_CAPABILITY_INVALID_DETAIL: &str = "containment PTY capability is invalid";

impl LinuxContainmentLease {
    pub fn bind_pty_proof(
        &mut self,
        proof: &PtySessionProof,
    ) -> Result<(), LinuxContainmentError> {
        if self.record.pty_device.is_some() || self.record.pty_capability.is_some() {
            return Err(LinuxContainmentError::Identity(PTY_ALREADY_BOUND_DETAIL));
        }
        let device = proof.linux_device();
        let capability = proof.linux_capability();
        if device == u64::default() {
            return Err(LinuxContainmentError::Identity(PTY_DEVICE_INVALID_DETAIL));
        }
        if capability.is_empty() {
            return Err(LinuxContainmentError::Identity(
                PTY_CAPABILITY_INVALID_DETAIL,
            ));
        }
        self.record.pty_device = Some(device);
        self.record.pty_capability = Some(capability.to_owned());
        Ok(())
    }

    pub(super) fn record_bound_root(&mut self, root: ProcessIdentity) {
        self.record.bound_pid = Some(root.pid);
        self.record.bound_start_time = Some(root.start_time);
    }
}
