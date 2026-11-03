use crate::agent::{FencedDescendants, ObservedProcess, ProcessSource};
use crate::process::ProcessIdentity;
use std::path::PathBuf;
use std::sync::{Arc, Condvar, Mutex};

const PEER_GENERATION_DELTA: u64 = 1;
const SOURCE_NOT_INSTALLED: &str = "Codex ingress process source is not installed";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum PeerEvidence {
    Direct,
    WrongGeneration,
    WrongLineage,
}

#[derive(Debug, Default)]
pub(super) struct LiveProcessSource {
    state: Mutex<LiveProcessState>,
}

#[derive(Debug, Default)]
pub(super) struct BlockingProcessSourceGate {
    inner: Arc<BlockingProcessSourceState>,
}

#[derive(Debug, Default)]
struct BlockingProcessSourceState {
    state: Mutex<BlockingGateState>,
    changed: Condvar,
}

#[derive(Debug, Default)]
struct LiveProcessState {
    agent: Option<ObservedProcess>,
    hook: Option<PathBuf>,
    peer: Option<ObservedProcess>,
    next_fenced_gate: Option<Arc<BlockingProcessSourceState>>,
}

#[derive(Debug, Default)]
struct BlockingGateState {
    entered: bool,
    released: bool,
    returned: bool,
}

impl LiveProcessSource {
    pub(super) fn install(&self, agent: ObservedProcess, hook: PathBuf) {
        let mut state = self.state.lock().unwrap();
        state.agent = Some(agent);
        state.hook = Some(hook);
        state.peer = None;
    }

    pub(super) fn register_peer(&self, peer: ProcessIdentity, evidence: PeerEvidence) {
        let mut state = self.state.lock().unwrap();
        let agent = state.agent.as_ref().expect(SOURCE_NOT_INSTALLED);
        let hook = state.hook.as_ref().expect(SOURCE_NOT_INSTALLED);
        let start_time = match evidence {
            PeerEvidence::WrongGeneration => peer.start_time.saturating_add(PEER_GENERATION_DELTA),
            PeerEvidence::Direct | PeerEvidence::WrongLineage => peer.start_time,
        };
        let parent_pid = match evidence {
            PeerEvidence::WrongLineage => agent.parent_pid,
            PeerEvidence::Direct | PeerEvidence::WrongGeneration => agent.pid,
        };
        state.peer = Some(ObservedProcess {
            pid: peer.pid,
            start_time,
            parent_pid,
            process_group: agent.process_group,
            foreground_group: agent.foreground_group,
            executable: hook.to_string_lossy().into_owned(),
            argv: vec![hook.to_string_lossy().into_owned()],
            environment: None,
            cwd: None,
            evidence_paths: None,
        });
    }

    pub(super) fn clear_peer(&self) {
        self.state.lock().unwrap().peer = None;
    }

    pub(super) fn gate_next_fenced_descendants(&self) -> Arc<BlockingProcessSourceGate> {
        let gate = Arc::new(BlockingProcessSourceGate::default());
        let previous = self
            .state
            .lock()
            .unwrap()
            .next_fenced_gate
            .replace(Arc::clone(&gate.inner));
        assert!(previous.is_none());
        gate
    }

    fn take_fenced_gate(&self) -> Option<Arc<BlockingProcessSourceState>> {
        self.state.lock().unwrap().next_fenced_gate.take()
    }

    fn snapshot(&self) -> anyhow::Result<Vec<ObservedProcess>> {
        let state = self.state.lock().unwrap();
        let agent = state
            .agent
            .clone()
            .ok_or_else(|| anyhow::anyhow!(SOURCE_NOT_INSTALLED))?;
        Ok(std::iter::once(agent)
            .chain(state.peer.iter().cloned())
            .collect())
    }
}

impl BlockingProcessSourceGate {
    pub(super) fn wait_until_entered(&self) {
        let state = self.inner.state.lock().unwrap();
        drop(
            self.inner
                .changed
                .wait_while(state, |state| !state.entered)
                .unwrap(),
        );
    }

    pub(super) fn release(&self) {
        self.inner.release();
    }

    pub(super) fn wait_until_returned(&self) {
        let state = self.inner.state.lock().unwrap();
        drop(
            self.inner
                .changed
                .wait_while(state, |state| !state.returned)
                .unwrap(),
        );
    }
}

impl Drop for BlockingProcessSourceGate {
    fn drop(&mut self) {
        self.inner.release();
    }
}

impl BlockingProcessSourceState {
    fn release(&self) {
        self.state.lock().unwrap().released = true;
        self.changed.notify_all();
    }

    fn block(&self) {
        let mut state = self.state.lock().unwrap();
        state.entered = true;
        self.changed.notify_all();
        drop(
            self.changed
                .wait_while(state, |state| !state.released)
                .unwrap(),
        );
    }

    fn mark_returned(&self) {
        self.state.lock().unwrap().returned = true;
        self.changed.notify_all();
    }
}

impl ProcessSource for LiveProcessSource {
    fn descendants(&self, _root_pid: u32) -> anyhow::Result<Vec<ObservedProcess>> {
        self.snapshot()
    }

    fn fenced_descendants(&self, _root: ProcessIdentity) -> anyhow::Result<FencedDescendants> {
        if let Some(gate) = self.take_fenced_gate() {
            gate.block();
            let snapshot = self.snapshot();
            gate.mark_returned();
            return snapshot.map(FencedDescendants::Stable);
        }
        Ok(FencedDescendants::Stable(self.snapshot()?))
    }
}
