use super::{LiveProcessIdentity, ProcessIdentity};
use anyhow::{Context, Result};
use std::fmt;
use std::process::{Child, ChildStderr, ChildStdin, ChildStdout, ExitStatus};

const CAPTURE_IDENTITY_CONTEXT: &str = "capture detached child identity";
const CAPTURE_LIVE_AUTHORITY_CONTEXT: &str = "capture detached child signal authority";
const CHILD_GONE_BEFORE_CAPTURE: &str = "detached child exited before custody capture";
const EXACT_AUTHORITY_UNAVAILABLE: &str = "detached child exact signal authority is unavailable";

pub struct DetachedChildCustody {
    child: Child,
    identity: Option<ProcessIdentity>,
    live: Option<LiveProcessIdentity>,
}

pub struct DetachedChildCustodyError {
    source: anyhow::Error,
    child: Child,
    identity: Option<ProcessIdentity>,
}

impl DetachedChildCustody {
    pub fn capture(child: Child) -> std::result::Result<Self, DetachedChildCustodyError> {
        let identity = match ProcessIdentity::capture(child.id()).context(CAPTURE_IDENTITY_CONTEXT)
        {
            Ok(identity) => identity,
            Err(source) => {
                return Err(DetachedChildCustodyError {
                    source,
                    child,
                    identity: None,
                });
            }
        };
        match Self::capture_live(identity) {
            Ok(live) => Ok(Self {
                child,
                identity: Some(identity),
                live: Some(live),
            }),
            Err(source) => Err(DetachedChildCustodyError {
                source,
                child,
                identity: Some(identity),
            }),
        }
    }

    fn capture_live(identity: ProcessIdentity) -> Result<LiveProcessIdentity> {
        LiveProcessIdentity::capture(identity)
            .context(CAPTURE_LIVE_AUTHORITY_CONTEXT)?
            .ok_or_else(|| anyhow::anyhow!(CHILD_GONE_BEFORE_CAPTURE))
    }

    pub fn identity(&self) -> Option<ProcessIdentity> {
        self.identity
    }

    pub(in crate::process) fn live_identity(&self) -> Option<LiveProcessIdentity> {
        self.live.clone()
    }

    #[cfg(test)]
    pub(in crate::process) fn discard_live_identity_for_test(&mut self) {
        self.live = None;
    }

    pub fn process_id(&self) -> u32 {
        self.child.id()
    }

    pub fn try_wait(&mut self) -> std::io::Result<Option<ExitStatus>> {
        self.child.try_wait()
    }

    pub(in crate::process) fn take_stdout(&mut self) -> Option<ChildStdout> {
        self.child.stdout.take()
    }

    pub(in crate::process) fn take_stdin(&mut self) -> Option<ChildStdin> {
        self.child.stdin.take()
    }

    pub(in crate::process) fn take_stderr(&mut self) -> Option<ChildStderr> {
        self.child.stderr.take()
    }

    pub fn wait(&mut self) -> std::io::Result<ExitStatus> {
        self.child.wait()
    }

    pub fn request_direct_termination(&mut self) -> std::io::Result<()> {
        ::std::process::Child::kill(&mut self.child)
    }

    pub fn request_force_termination(&self) -> Result<bool> {
        let live = self
            .live
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!(EXACT_AUTHORITY_UNAVAILABLE))?;
        super::signal_authority::signal_exact_process(
            live,
            super::signal_authority::ExactProcessSignal::ForceTermination,
        )
    }

    pub(in crate::process) fn release(self) -> Child {
        self.child
    }
}

impl DetachedChildCustodyError {
    pub fn into_parts(self) -> (anyhow::Error, Child) {
        (self.source, self.child)
    }

    pub fn into_custody(self) -> DetachedChildCustody {
        DetachedChildCustody {
            child: self.child,
            identity: self.identity,
            live: None,
        }
    }

    pub(in crate::process) fn into_source_and_custody(
        self,
    ) -> (anyhow::Error, DetachedChildCustody) {
        let custody = DetachedChildCustody {
            child: self.child,
            identity: self.identity,
            live: None,
        };
        (self.source, custody)
    }
}

impl fmt::Debug for DetachedChildCustodyError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("DetachedChildCustodyError")
            .field("source", &self.source)
            .field("child_id", &self.child.id())
            .finish()
    }
}

impl fmt::Display for DetachedChildCustodyError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.source.fmt(formatter)
    }
}

impl std::error::Error for DetachedChildCustodyError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(self.source.as_ref())
    }
}
