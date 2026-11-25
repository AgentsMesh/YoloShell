use crate::worker::tests::*;
struct ImmediateWriteFailure;

pub(super) struct WriteZero;

impl Write for WriteZero {
    fn write(&mut self, _data: &[u8]) -> io::Result<usize> {
        Ok(0)
    }

    fn flush(&mut self) -> io::Result<()> {
        unreachable!()
    }
}

impl Write for ImmediateWriteFailure {
    fn write(&mut self, _data: &[u8]) -> io::Result<usize> {
        Err(io::ErrorKind::BrokenPipe.into())
    }

    fn flush(&mut self) -> io::Result<()> {
        unreachable!()
    }
}

struct PartialWriteFailure {
    wrote_prefix: bool,
}

impl Write for PartialWriteFailure {
    fn write(&mut self, _data: &[u8]) -> io::Result<usize> {
        if self.wrote_prefix {
            Err(io::ErrorKind::BrokenPipe.into())
        } else {
            self.wrote_prefix = true;
            Ok(1)
        }
    }

    fn flush(&mut self) -> io::Result<()> {
        unreachable!()
    }
}

struct InterruptedOnce {
    interrupted: bool,
    bytes: Vec<u8>,
}

impl Write for InterruptedOnce {
    fn write(&mut self, data: &[u8]) -> io::Result<usize> {
        if !self.interrupted {
            self.interrupted = true;
            return Err(io::ErrorKind::Interrupted.into());
        }
        self.bytes.extend_from_slice(data);
        Ok(data.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

#[test]
fn bootstrap_delivery_classifies_every_write_boundary() {
    let mut unused = Vec::new();
    let not_required = deliver_bootstrap_input(&mut unused, None);
    assert_eq!(not_required.delivery, BootstrapDelivery::NotRequired);

    let empty = deliver_bootstrap_input(&mut unused, Some(&[]));
    assert_eq!(empty.delivery, BootstrapDelivery::Failed);

    let mut immediate = ImmediateWriteFailure;
    let failed = deliver_bootstrap_input(&mut immediate, Some(b"resume\n"));
    assert_eq!(failed.delivery, BootstrapDelivery::Failed);

    let mut write_zero = WriteZero;
    let write_zero = deliver_bootstrap_input(&mut write_zero, Some(b"resume\n"));
    assert_eq!(write_zero.delivery, BootstrapDelivery::Failed);

    let mut partial = PartialWriteFailure {
        wrote_prefix: false,
    };
    let partial = deliver_bootstrap_input(&mut partial, Some(b"resume\n"));
    assert_eq!(partial.delivery, BootstrapDelivery::Unknown);

    let mut flush = FlushFails;
    let flush = deliver_bootstrap_input(&mut flush, Some(b"resume\n"));
    assert_eq!(flush.delivery, BootstrapDelivery::Unknown);

    let mut interrupted = InterruptedOnce {
        interrupted: false,
        bytes: Vec::new(),
    };
    let delivered = deliver_bootstrap_input(&mut interrupted, Some(b"resume\n"));
    assert_eq!(delivered.delivery, BootstrapDelivery::Delivered);
    assert_eq!(interrupted.bytes, b"resume\n");
}

#[tokio::test]
async fn readiness_fences_lease_and_input_admission() {
    let fixture = Fixture::new();
    fixture.state.ready.store(false, Ordering::Release);
    let error = fixture
        .state
        .acquire_lease(
            fixture.controller(),
            &fixture.lease_request("not-ready"),
            false,
        )
        .await
        .unwrap_err();
    assert_eq!(error.code, ErrorBody::conflict("").code);
    assert_eq!(fixture.state.status().status, TerminalRuntimeStatus::Starting);

    fixture.state.ready.store(true, Ordering::Release);
    let lease = fixture
        .state
        .acquire_lease(fixture.controller(), &fixture.lease_request("ready"), false)
        .await
        .unwrap();
    let result = fixture
        .state
        .write_input(
            fixture.controller(),
            &InputRequest {
                terminal_id: fixture.terminal_id(),
                runtime_epoch: fixture.state.runtime_epoch(),
                generation: lease.generation,
                writer_capability: lease.writer_capability.clone(),
                input_id: Uuid::new_v4(),
                data: b"accepted-after-ready\n".to_vec(),
            },
        )
        .await
        .unwrap();
    assert!(!result.duplicate);
}
