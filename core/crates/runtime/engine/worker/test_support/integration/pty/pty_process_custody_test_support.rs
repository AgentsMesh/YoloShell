use super::PtyProcessCustody;
use crate::process::{CapturedProcessTreeStateSnapshot, ProcessIdentity};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum PtyProcessCustodyKind {
    Tracking,
    Freezing,
    Frozen,
    Killing,
    Survivor,
    GonePending,
    GoneReported,
    MasterClosePending,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct PtyProcessCustodySnapshot {
    pub(crate) kind: PtyProcessCustodyKind,
    pub(crate) root: ProcessIdentity,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct PtyProcessCustodyStateSnapshot {
    pub(crate) kind: PtyProcessCustodyKind,
    pub(crate) root: ProcessIdentity,
    pub(crate) tree: Option<CapturedProcessTreeStateSnapshot>,
    pub(crate) freeze_confirmation: Option<String>,
}

impl PtyProcessCustody {
    pub(crate) fn snapshot_for_test(&self) -> PtyProcessCustodySnapshot {
        let (kind, root) = match self {
            Self::Tracking(tree) => (PtyProcessCustodyKind::Tracking, tree.root()),
            Self::Freezing(tree) => (PtyProcessCustodyKind::Freezing, tree.root()),
            Self::Frozen(tree) => (PtyProcessCustodyKind::Frozen, tree.root()),
            Self::Killing(tree) => (PtyProcessCustodyKind::Killing, tree.root()),
            Self::Survivor(tree) => (PtyProcessCustodyKind::Survivor, tree.root()),
            Self::GonePending(completion) => {
                (PtyProcessCustodyKind::GonePending, completion.root())
            }
            Self::GoneReported(completion) => {
                (PtyProcessCustodyKind::GoneReported, completion.root())
            }
            Self::MasterClosePending(completion) => {
                (PtyProcessCustodyKind::MasterClosePending, completion.root())
            }
        };
        PtyProcessCustodySnapshot { kind, root }
    }

    pub(crate) fn state_snapshot_for_test(&self) -> PtyProcessCustodyStateSnapshot {
        let basic = self.snapshot_for_test();
        let tree = match self {
            Self::Tracking(tree) => Some(tree.state_snapshot_for_test()),
            Self::Freezing(tree) => Some(tree.state_snapshot_for_test()),
            Self::Frozen(tree) => Some(tree.state_snapshot_for_test()),
            Self::Killing(tree) => Some(tree.state_snapshot_for_test()),
            Self::Survivor(tree) => Some(tree.state_snapshot_for_test()),
            Self::GonePending(_) | Self::GoneReported(_) | Self::MasterClosePending(_) => None,
        };
        let freeze_confirmation = match self {
            Self::Freezing(tree) => Some(tree.confirmation_for_test()),
            _ => None,
        };
        PtyProcessCustodyStateSnapshot {
            kind: basic.kind,
            root: basic.root,
            tree,
            freeze_confirmation,
        }
    }
}
