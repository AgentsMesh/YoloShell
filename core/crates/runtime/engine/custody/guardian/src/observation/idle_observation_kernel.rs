use super::ChangeObservation;
use crate::paths::Paths;
use crate::pty_custody::{CustodyHolderIdentity, CustodyRuntimeIdentity};
use anyhow::Result;
use std::collections::BTreeSet;

#[path = "idle_observation_queue.rs"]
mod queue;
#[path = "idle_observation_vnode.rs"]
mod vnode;

pub(in crate::custody_guardian::settlement) struct KernelObservation {
    queue: queue::ObservationQueue,
    vnodes: Vec<vnode::ObservedVnode>,
    processes: Vec<CustodyHolderIdentity>,
}

impl KernelObservation {
    pub(super) fn capture(
        paths: &Paths,
        runtime: &CustodyRuntimeIdentity,
        holder: CustodyHolderIdentity,
    ) -> Result<Self> {
        Self::watch(
            &[
                paths.pty_custody(runtime.terminal_id),
                paths.runtime_record(runtime.terminal_id),
            ],
            vec![
                holder,
                CustodyHolderIdentity {
                    pid: runtime.worker_pid,
                    start_time: runtime.worker_start_time,
                },
                CustodyHolderIdentity {
                    pid: runtime.child_pid,
                    start_time: runtime.child_start_time,
                },
            ],
        )
    }

    fn watch(paths: &[std::path::PathBuf], processes: Vec<CustodyHolderIdentity>) -> Result<Self> {
        let queue = queue::ObservationQueue::create()?;
        let mut targets = BTreeSet::new();
        for path in paths {
            let path = crate::paths::platform::platform_path(path);
            targets.extend(path.ancestors().map(std::path::Path::to_path_buf));
        }
        let mut vnodes = Vec::with_capacity(targets.len());
        for target in targets {
            let vnode = vnode::ObservedVnode::open(target)?;
            vnode.register(&queue)?;
            vnodes.push(vnode);
        }
        for process in &processes {
            queue.watch_process(*process)?;
        }
        Ok(Self {
            queue,
            vnodes,
            processes,
        })
    }
}

impl ChangeObservation for KernelObservation {
    fn unchanged(&self) -> Result<bool> {
        self.queue.unchanged()
    }

    fn finish_validation(&self) -> Result<bool> {
        for vnode in &self.vnodes {
            if !vnode.matches_current_path()? {
                return Ok(false);
            }
        }
        for process in &self.processes {
            if !process.matches() {
                return Ok(false);
            }
        }
        self.unchanged()
    }
}

#[cfg(all(test, feature = "yolo-integration-tests"))]
#[path = "../../integration_tests/idle_observation_kernel_tests.rs"]
mod tests;
