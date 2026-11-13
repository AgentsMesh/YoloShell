pub(crate) struct ControllerClaimProbe {
    pub(crate) token: Uuid,
    pub(crate) home: PathBuf,
    pub(crate) reached: std::sync::mpsc::Sender<ControllerClaimProgress>,
    pub(crate) release: std::sync::mpsc::Receiver<()>,
}

#[derive(Debug)]
pub(crate) enum ControllerClaimProgress {
    Claimed(u64),
    OpenFailed(String),
}

pub(crate) struct ControllerClaimProbeRegistration {
    pub(crate) token: Uuid,
}

pub(crate) static CONTROLLER_CLAIM_PROBE: std::sync::OnceLock<Mutex<Option<ControllerClaimProbe>>> =
    std::sync::OnceLock::new();

pub(crate) fn install_controller_claim_probe(
    home: &Path,
    reached: std::sync::mpsc::Sender<ControllerClaimProgress>,
    release: std::sync::mpsc::Receiver<()>,
) -> ControllerClaimProbeRegistration {
    let token = Uuid::new_v4();
    let slot = CONTROLLER_CLAIM_PROBE.get_or_init(|| Mutex::new(None));
    let mut slot = lock_recover(slot);
    assert!(slot.is_none(), "controller claim probe already installed");
    *slot = Some(ControllerClaimProbe {
        token,
        home: Paths::new(home).home().to_path_buf(),
        reached,
        release,
    });
    ControllerClaimProbeRegistration { token }
}

impl Drop for ControllerClaimProbeRegistration {
    fn drop(&mut self) {
        let Some(slot) = CONTROLLER_CLAIM_PROBE.get() else {
            return;
        };
        let mut slot = lock_recover(slot);
        if slot.as_ref().is_some_and(|probe| probe.token == self.token) {
            slot.take();
        }
    }
}

pub(crate) fn pause_after_controller_claim(home: &Path, generation: u64) {
    let Some(slot) = CONTROLLER_CLAIM_PROBE.get() else {
        return;
    };
    let probe = {
        let mut slot = lock_recover(slot);
        if slot.as_ref().is_some_and(|probe| probe.home == home) {
            slot.take()
        } else {
            None
        }
    };
    let Some(probe) = probe else {
        return;
    };
    if probe
        .reached
        .send(ControllerClaimProgress::Claimed(generation))
        .is_ok()
    {
        let _ = probe.release.recv();
    }
}
