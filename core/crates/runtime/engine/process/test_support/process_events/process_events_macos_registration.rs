use crate::process::LiveProcessIdentity;
use anyhow::Result;

type RegistrationAction = Box<dyn FnOnce() + Send + Sync>;

#[derive(Default)]
pub(super) struct RegistrationFaults {
    missing_before_add: Option<LiveProcessIdentity>,
    retire_after_add: Option<(LiveProcessIdentity, RegistrationAction)>,
}

impl RegistrationFaults {
    pub(super) fn fail_as_missing(&mut self, identity: LiveProcessIdentity) {
        self.missing_before_add = Some(identity);
    }

    pub(super) fn retire_after_add(
        &mut self,
        identity: LiveProcessIdentity,
        action: impl FnOnce() + Send + Sync + 'static,
    ) {
        self.retire_after_add = Some((identity, Box::new(action)));
    }

    pub(super) fn before_change(&mut self, change: &libc::kevent) -> Result<()> {
        let Some(identity) = self.missing_before_add else {
            return Ok(());
        };
        if !matches_add(change, identity) {
            return Ok(());
        }
        self.missing_before_add.take();
        Err(std::io::Error::from_raw_os_error(libc::ESRCH).into())
    }

    pub(super) fn after_change(
        &mut self,
        change: &libc::kevent,
    ) -> Option<(LiveProcessIdentity, RegistrationAction)> {
        let (expected, _) = self.retire_after_add.as_ref()?;
        if !matches_add(change, *expected) {
            return None;
        }
        self.retire_after_add.take()
    }
}

fn matches_add(change: &libc::kevent, identity: LiveProcessIdentity) -> bool {
    change.filter == libc::EVFILT_PROC
        && change.flags & libc::EV_ADD != u16::default()
        && change.ident == identity.pid() as usize
}
