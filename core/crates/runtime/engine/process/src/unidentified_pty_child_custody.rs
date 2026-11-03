use super::{OwnedPtyChild, WaitableChildSpawnBoundary};
use portable_pty::ExitStatus;
use std::io;

pub struct UnidentifiedPtyChildCustody<'boundary, 'spawn> {
    child: OwnedPtyChild,
    _boundary: &'boundary WaitableChildSpawnBoundary<'spawn>,
}

pub fn retain_unidentified_child<'boundary, 'spawn>(
    boundary: &'boundary WaitableChildSpawnBoundary<'spawn>,
    child: OwnedPtyChild,
) -> UnidentifiedPtyChildCustody<'boundary, 'spawn> {
    UnidentifiedPtyChildCustody {
        child,
        _boundary: boundary,
    }
}

impl UnidentifiedPtyChildCustody<'_, '_> {
    pub fn request_direct_termination(&mut self) -> io::Result<()> {
        self.child.kill()
    }

    pub fn try_wait(&mut self) -> io::Result<Option<ExitStatus>> {
        self.child.try_wait()
    }
}
