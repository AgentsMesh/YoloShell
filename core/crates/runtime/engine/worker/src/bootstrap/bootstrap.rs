#[derive(Debug)]
pub(crate) struct BootstrapAttempt {
    pub(crate) delivery: BootstrapDelivery,
    pub(crate) error: Option<io::Error>,
}

impl BootstrapAttempt {
    pub(crate) fn success(delivery: BootstrapDelivery) -> Self {
        Self {
            delivery,
            error: None,
        }
    }

    pub(crate) fn failure(delivery: BootstrapDelivery, error: io::Error) -> Self {
        Self {
            delivery,
            error: Some(error),
        }
    }
}

pub(crate) fn deliver_bootstrap_input(
    writer: &mut dyn Write,
    input: Option<&[u8]>,
) -> BootstrapAttempt {
    #[cfg(feature = "adversarial-test-hooks")]
    if crate::survivor_test_hook::bootstrap_failure_enabled() {
        return BootstrapAttempt::failure(
            BootstrapDelivery::Failed,
            crate::survivor_test_hook::bootstrap_failure(),
        );
    }
    let Some(input) = input else {
        return BootstrapAttempt::success(BootstrapDelivery::NotRequired);
    };
    if input.is_empty() {
        return BootstrapAttempt::failure(
            BootstrapDelivery::Failed,
            io::Error::new(io::ErrorKind::InvalidInput, BOOTSTRAP_INPUT_EMPTY_DETAIL),
        );
    }
    let mut written = 0_usize;
    while written < input.len() {
        match writer.write(&input[written..]) {
            Ok(0) => {
                let error = io::Error::new(io::ErrorKind::WriteZero, BOOTSTRAP_WRITE_ZERO_DETAIL);
                return BootstrapAttempt::failure(bootstrap_failure(written), error);
            }
            Ok(length) => written += length,
            Err(error) if error.kind() == io::ErrorKind::Interrupted => {}
            Err(error) => return BootstrapAttempt::failure(bootstrap_failure(written), error),
        }
    }
    match writer.flush() {
        Ok(()) => BootstrapAttempt::success(BootstrapDelivery::Delivered),
        Err(error) => BootstrapAttempt::failure(BootstrapDelivery::Unknown, error),
    }
}

pub(crate) fn bootstrap_failure(written: usize) -> BootstrapDelivery {
    if written == 0 {
        BootstrapDelivery::Failed
    } else {
        BootstrapDelivery::Unknown
    }
}

#[cfg(test)]
pub(crate) async fn write_initial_input(state: Arc<WorkerState>, input: Vec<u8>) -> Result<()> {
    let mut writer = lock_recover(&state.writer);
    let writer = writer
        .as_mut()
        .context("PTY writer unavailable for bootstrap input")?;
    let attempt = deliver_bootstrap_input(writer, Some(&input));
    match attempt.error {
        Some(error) => Err(error.into()),
        None => Ok(()),
    }
}

#[cfg(all(test, target_os = "macos"))]
pub(crate) fn receive_bootstrap_writer_once_for_test(
    socket: libc::c_int,
) -> Result<process::MacosProcessCapabilityChildWriter> {
    bootstrap_gate::descriptor::receive_writer_once_for_test(socket)
}

#[cfg(all(test, target_os = "macos"))]
pub(crate) fn send_bootstrap_writer_once_for_test(
    socket: libc::c_int,
    descriptor: libc::c_int,
) -> Result<()> {
    bootstrap_gate::descriptor::send_writer_once_for_test(socket, descriptor)
}
