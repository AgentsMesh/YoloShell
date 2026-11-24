use super::*;

pub(crate) struct SpawnedHost {
    pub(super) child: Child,
    pub(super) home: PathBuf,
    pub(super) log: host_log::HostLog,
}

impl SpawnedHost {
    pub(super) fn log_identity(&self) -> host_log::HostLogIdentity {
        self.log.identity()
    }
}

pub(crate) fn prepare_host_home(home: &Path) {
    Paths::new(home)
        .ensure_base()
        .expect("prepare marked private test Host home");
}

pub(crate) fn spawn_logged_host(home: &Path, command: &mut Command) -> SpawnedHost {
    prepare_host_home(home);
    let log = host_log::HostLog::open(home).expect("create private Host stderr log");
    let stderr = log.child_stderr().expect("clone Host stderr log");
    let child = command.stderr(stderr).spawn().expect("spawn test Host");
    SpawnedHost {
        child,
        home: home.to_path_buf(),
        log,
    }
}
