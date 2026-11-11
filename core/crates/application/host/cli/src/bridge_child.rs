use std::process::Child;
use yolo_runtime::process::{
    DetachedStandardError, ReservedDetachedChildSettlement, reserve_detached_child_settlement,
    spawn_self_detaching,
};

const HOST_POLL_FAILED: &str = "host child wait authority was lost during cleanup";
const HOST_TERMINATION_FAILED: &str = "host child termination failed during cleanup";
const HOST_CHILD_SETTLEMENT_MISSING: &str = "host child cleanup reservation is missing";

pub(super) trait HostChildStatus {
    fn process_id(&self) -> Option<u32>;
    fn try_wait_status(&mut self) -> std::io::Result<Option<String>>;
    fn preserve(&mut self);
}

pub(super) struct SpawnedHost {
    child: Option<Child>,
    settlement: Option<ReservedDetachedChildSettlement>,
    #[cfg(test)]
    fail_status_poll: bool,
    #[cfg(test)]
    fail_drop_poll: bool,
    #[cfg(test)]
    fail_drop_termination: bool,
}

impl SpawnedHost {
    pub(super) fn spawn(
        executable: &std::path::Path,
        args: &[std::ffi::OsString],
    ) -> anyhow::Result<Self> {
        let settlement = reserve_detached_child_settlement()?;
        let child = spawn_self_detaching(executable, args, DetachedStandardError::Null)?;
        Ok(Self::retaining(child, settlement))
    }

    fn retaining(child: Child, settlement: ReservedDetachedChildSettlement) -> Self {
        Self {
            child: Some(child),
            settlement: Some(settlement),
            #[cfg(test)]
            fail_status_poll: false,
            #[cfg(test)]
            fail_drop_poll: false,
            #[cfg(test)]
            fail_drop_termination: false,
        }
    }

    #[cfg(test)]
    pub(super) fn new(child: Child) -> Self {
        Self::retaining(child, reserve_detached_child_settlement().unwrap())
    }

    #[cfg(test)]
    pub(super) fn with_poll_errors(child: Child) -> Self {
        let mut spawned = Self::new(child);
        spawned.fail_status_poll = true;
        spawned.fail_drop_poll = true;
        spawned
    }

    #[cfg(test)]
    pub(super) fn with_termination_error(child: Child) -> Self {
        let mut spawned = Self::new(child);
        spawned.fail_drop_termination = true;
        spawned
    }
}

impl HostChildStatus for SpawnedHost {
    fn process_id(&self) -> Option<u32> {
        self.child.as_ref().map(Child::id)
    }

    fn try_wait_status(&mut self) -> std::io::Result<Option<String>> {
        #[cfg(test)]
        if std::mem::take(&mut self.fail_status_poll) {
            return Err(std::io::ErrorKind::Other.into());
        }
        let Some(child) = self.child.as_mut() else {
            return Ok(None);
        };
        match child.try_wait()? {
            Some(status) => {
                self.child.take();
                Ok(Some(status.to_string()))
            }
            None => Ok(None),
        }
    }

    fn preserve(&mut self) {
        self.child.take();
        self.settlement.take();
    }
}

impl Drop for SpawnedHost {
    fn drop(&mut self) {
        let Some(mut child) = self.child.take() else {
            return;
        };
        let settlement = self.settlement.take().expect(HOST_CHILD_SETTLEMENT_MISSING);
        #[cfg(test)]
        if std::mem::take(&mut self.fail_drop_poll) {
            settle_failed_child(
                child,
                std::io::ErrorKind::Other.into(),
                HOST_POLL_FAILED,
                settlement,
            );
            return;
        }
        let observed = ::std::process::Child::try_wait(&mut child);
        match observed {
            Ok(Some(_)) => {}
            Ok(None) => {
                #[cfg(test)]
                if std::mem::take(&mut self.fail_drop_termination) {
                    settle_failed_child(
                        child,
                        std::io::ErrorKind::Other.into(),
                        HOST_TERMINATION_FAILED,
                        settlement,
                    );
                    return;
                }
                let terminated = ::std::process::Child::kill(&mut child);
                if let Err(error) = terminated {
                    settle_failed_child(child, error, HOST_TERMINATION_FAILED, settlement);
                    return;
                }
                if let Err(error) = ::std::process::Child::wait(&mut child) {
                    settle_failed_child(child, error, HOST_POLL_FAILED, settlement);
                }
            }
            Err(error) => settle_failed_child(child, error, HOST_POLL_FAILED, settlement),
        }
    }
}

fn settle_failed_child(
    child: Child,
    error: std::io::Error,
    message: &'static str,
    settlement: ReservedDetachedChildSettlement,
) {
    tracing::warn!(process_id = child.id(), %error, message);
    settlement.terminate(child);
}

#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-unit-tests"))] #[path = "../unit_tests/bridge_child_tests.rs"]
mod tests;
