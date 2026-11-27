use crate::process::{MacosPassivePtyMaster, PtyMasterFdCustodyProbe, PtySessionProof};
use portable_pty::{PtySize, native_pty_system};
use std::os::fd::AsRawFd;
use std::os::unix::process::CommandExt;

const FCNTL_FAILURE: libc::c_int = -1;

#[doc(hidden)]
pub struct RetainedPtyProbeFixture(MacosPassivePtyMaster);

impl RetainedPtyProbeFixture {
    pub fn new(size: PtySize, capability: &str) -> (Self, PtyMasterFdCustodyProbe) {
        let pair = native_pty_system().openpty(size).unwrap();
        let proof =
            PtySessionProof::capture_bound(pair.master.as_ref(), capability.to_owned()).unwrap();
        let master = MacosPassivePtyMaster::duplicate_exact(&proof).unwrap();
        let probe = PtyMasterFdCustodyProbe::capture(&master).unwrap();
        (Self(master), probe)
    }

    pub fn configure_exec_inheritance(&self, command: &mut std::process::Command) {
        let descriptor = self.0.descriptor().as_raw_fd();
        unsafe {
            command.pre_exec(move || {
                let flags = libc::fcntl(descriptor, libc::F_GETFD);
                if flags == FCNTL_FAILURE
                    || libc::fcntl(descriptor, libc::F_SETFD, flags & !libc::FD_CLOEXEC)
                        == FCNTL_FAILURE
                {
                    Err(std::io::Error::last_os_error())
                } else {
                    Ok(())
                }
            });
        }
    }
}
