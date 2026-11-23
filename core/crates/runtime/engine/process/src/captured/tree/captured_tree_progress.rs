use super::{
    CapturedProcessTree, FreezingProcessTree, FrozenProcessTree, KillPendingProcessTree,
    TerminationUnsupportedProcessTree,
};

impl CapturedProcessTree {
    pub fn take_capability_scan_activity(&mut self) -> super::CapabilityScanActivity {
        std::mem::take(&mut self.capability_scan_activity)
    }

    #[cfg(any(test, feature = "runtime-component-tests"))]
    pub fn record_capability_scan_activity_for_test(
        &mut self,
        activity: super::CapabilityScanActivity,
    ) {
        self.capability_scan_activity = self.capability_scan_activity.merge(activity);
    }
}

impl FreezingProcessTree {
    pub fn take_capability_scan_activity(&mut self) -> super::CapabilityScanActivity {
        self.tree.take_capability_scan_activity()
    }
}

impl FrozenProcessTree {
    pub fn take_capability_scan_activity(&mut self) -> super::CapabilityScanActivity {
        self.tree.take_capability_scan_activity()
    }
}

impl KillPendingProcessTree {
    pub fn take_capability_scan_activity(&mut self) -> super::CapabilityScanActivity {
        self.tree.take_capability_scan_activity()
    }
}

impl TerminationUnsupportedProcessTree {
    pub fn take_capability_scan_activity(&mut self) -> super::CapabilityScanActivity {
        self.tree.take_capability_scan_activity()
    }
}
