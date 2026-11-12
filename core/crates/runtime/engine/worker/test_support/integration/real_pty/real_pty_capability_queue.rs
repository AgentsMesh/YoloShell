#[cfg(target_os = "macos")]
const FIXTURE_QUEUED_WRITER_MISSING_DETAIL: &str =
    "fixture queued process capability writer is missing";
#[cfg(target_os = "macos")]
const FIXTURE_QUEUED_WRITER_VISIBLE_DETAIL: &str =
    "queued process capability writer leaked into the manager descriptor table";
#[cfg(target_os = "macos")]
const FIXTURE_QUEUED_WRITER_RELEASED_DETAIL: &str =
    "queued process capability writer was mistaken for released custody";
#[cfg(target_os = "macos")]
struct FixtureQueuedPtyCapability {
    receiver: Option<std::os::unix::net::UnixStream>,
    identity: process::MacosProcessCapabilityIdentity,
}

#[cfg(target_os = "macos")]
impl FixtureQueuedPtyCapability {
    fn new(writer: process::MacosProcessCapabilityChildWriter) -> Self {
        use std::os::fd::AsRawFd as _;
        let identity = writer.identity();
        let (sender, receiver) = std::os::unix::net::UnixStream::pair().unwrap();
        crate::worker::send_bootstrap_writer_once_for_test(sender.as_raw_fd(), writer.descriptor())
            .unwrap();
        drop(writer);
        drop(sender);
        Self {
            receiver: Some(receiver),
            identity,
        }
    }

    fn assert_hidden_and_present(&self, authority: &process::MacosProcessCapabilityManagerReader) {
        assert!(
            !process::process_holds_capability_for_test(
                std::process::id(),
                self.identity.writer_handle(),
            )
            .unwrap(),
            "{FIXTURE_QUEUED_WRITER_VISIBLE_DETAIL}"
        );
        assert_eq!(
            authority.poll_release_for_test().unwrap(),
            process::MacosProcessCapabilityRelease::WriterPresent,
            "{FIXTURE_QUEUED_WRITER_RELEASED_DETAIL}"
        );
    }

    fn release(&mut self) {
        let receiver = self
            .receiver
            .take()
            .expect(FIXTURE_QUEUED_WRITER_MISSING_DETAIL);
        drop(receiver);
    }
}

impl FixturePtyCapability {
    pub(super) fn queued_descriptor() -> Self {
        #[cfg(target_os = "macos")]
        let (authority, writer) = process::MacosProcessCapabilityIssuer::create()
            .unwrap()
            .transfer_for_test()
            .unwrap();
        #[cfg(target_os = "macos")]
        let queued = FixtureQueuedPtyCapability::new(writer);
        Self {
            value: Uuid::new_v4().to_string(),
            #[cfg(target_os = "macos")]
            authority,
            #[cfg(target_os = "macos")]
            holder: None,
            #[cfg(target_os = "macos")]
            queued: Some(queued),
        }
    }

    pub(super) fn assert_queued_writer_present(&self) {
        #[cfg(target_os = "macos")]
        self.queued
            .as_ref()
            .expect(FIXTURE_QUEUED_WRITER_MISSING_DETAIL)
            .assert_hidden_and_present(&self.authority);
        #[cfg(not(target_os = "macos"))]
        let _ = self;
    }
}
