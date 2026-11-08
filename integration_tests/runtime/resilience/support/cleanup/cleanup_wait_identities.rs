use super::*;

pub(super) fn wait_for_runtime_identities(
    identities: &[RuntimeProcessIdentity],
    timeout: Duration,
) {
    let deadline = Instant::now() + timeout;
    while identities
        .iter()
        .any(|identity| identity.observe() != ProcessIdentityObservation::Gone)
        && Instant::now() < deadline
    {
        std::thread::sleep(CLEANUP_POLL_INTERVAL);
    }
}

pub(super) fn signal_identity(identity: RuntimeProcessIdentity, signal: i32) -> Result<bool> {
    match identity.signal(signal) {
        Ok(delivered) => Ok(delivered),
        Err(_) if identity.observe() == ProcessIdentityObservation::Gone => Ok(false),
        Err(error) => {
            Err(error).with_context(|| format!("{SIGNAL_IDENTITY_CONTEXT}: {identity:?}"))
        }
    }
}
