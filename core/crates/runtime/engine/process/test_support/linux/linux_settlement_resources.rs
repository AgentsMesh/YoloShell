use crate::process::ScopedWaitableChild;
use crate::runtime_constants::{
    TEST_HARNESS_EXACT_ARGUMENT, TEST_HARNESS_NOCAPTURE_ARGUMENT,
    TEST_HARNESS_SINGLE_THREAD_ARGUMENT,
};
use std::io;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

pub(super) const ROLE_ENVIRONMENT: &str = "YOLO_SETTLEMENT_RESOURCE_TEST_ROLE";
pub(super) const HELPER_ROLE: &str = "descriptor-pressure";
pub(super) const COMPLETION_ENVIRONMENT: &str = "YOLO_SETTLEMENT_RESOURCE_TEST_COMPLETION";
const COMPLETION_FILE: &str = "completed";
const HELPER_TEST: &str =
    "linux_containment::kernel_tests::settlement_witness_resources::descriptor_pressure_helper";
const HELPER_TIMEOUT: Duration = Duration::from_secs(10);
const SYSTEM_SUCCESS: libc::c_int = 0;
const ONE_DESCRIPTOR_SLOT: libc::rlim_t = 1;

pub(super) async fn run_helper() {
    let directory = tempfile::tempdir().unwrap();
    let completion = directory.path().join(COMPLETION_FILE);
    let mut command = Command::new(std::env::current_exe().unwrap());
    command
        .args([
            TEST_HARNESS_EXACT_ARGUMENT,
            HELPER_TEST,
            TEST_HARNESS_NOCAPTURE_ARGUMENT,
            TEST_HARNESS_SINGLE_THREAD_ARGUMENT,
        ])
        .env(ROLE_ENVIRONMENT, HELPER_ROLE)
        .env(COMPLETION_ENVIRONMENT, &completion)
        .stdin(Stdio::null())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit());
    let deadline = Instant::now() + HELPER_TIMEOUT;
    ScopedWaitableChild::spawn_before_deadline(&mut command, deadline)
        .unwrap()
        .wait_for_clean_exit(deadline)
        .await
        .unwrap();
    assert!(
        completion.is_file(),
        "descriptor-pressure helper did not execute"
    );
}

pub(super) struct DescriptorPressure(libc::rlimit);

impl DescriptorPressure {
    // Only the isolated helper may lower its limit; existing handles remain valid.
    pub(super) fn enter(occupied_lowest_descriptor: libc::c_int) -> io::Result<Self> {
        // The caller fills the lowest free slot and retains it until restoration.
        // This exhausts allocation while keeping F_DUPFD_CLOEXEC's minimum valid.
        Self::one_free_slot(occupied_lowest_descriptor)
    }

    pub(super) fn one_free_slot(lowest_free_descriptor: libc::c_int) -> io::Result<Self> {
        let descriptor =
            libc::rlim_t::try_from(lowest_free_descriptor).map_err(io::Error::other)?;
        Self::with_soft_limit(descriptor + ONE_DESCRIPTOR_SLOT)
    }

    fn with_soft_limit(soft_limit: libc::rlim_t) -> io::Result<Self> {
        let mut original = libc::rlimit {
            rlim_cur: libc::rlim_t::default(),
            rlim_max: libc::rlim_t::default(),
        };
        if unsafe { libc::getrlimit(libc::RLIMIT_NOFILE, &mut original) } != SYSTEM_SUCCESS {
            return Err(io::Error::last_os_error());
        }
        let limited = libc::rlimit {
            rlim_cur: soft_limit,
            rlim_max: original.rlim_max,
        };
        if unsafe { libc::setrlimit(libc::RLIMIT_NOFILE, &limited) } != SYSTEM_SUCCESS {
            return Err(io::Error::last_os_error());
        }
        Ok(Self(original))
    }
}

impl Drop for DescriptorPressure {
    fn drop(&mut self) {
        assert_eq!(
            unsafe { libc::setrlimit(libc::RLIMIT_NOFILE, &self.0) },
            SYSTEM_SUCCESS,
            "restore descriptor limit: {}",
            io::Error::last_os_error(),
        );
    }
}
