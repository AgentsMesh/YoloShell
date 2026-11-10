use super::*;
use std::ops::{Deref, DerefMut};
use std::process::ExitStatus;

pub(crate) struct ChildGuard {
    child: Option<Child>,
}

impl ChildGuard {
    pub(crate) fn new(child: Child) -> Self {
        Self { child: Some(child) }
    }

    pub(crate) fn wait(&mut self) -> std::io::Result<ExitStatus> {
        self.child.as_mut().expect("guarded child").wait()
    }
}

impl Deref for ChildGuard {
    type Target = Child;

    fn deref(&self) -> &Self::Target {
        self.child.as_ref().expect("guarded child")
    }
}

impl DerefMut for ChildGuard {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.child.as_mut().expect("guarded child")
    }
}

impl Drop for ChildGuard {
    fn drop(&mut self) {
        let Some(mut child) = self.child.take() else {
            return;
        };
        if child.try_wait().ok().flatten().is_none() {
            let _ = child.kill();
        }
        let _ = child.wait();
    }
}
