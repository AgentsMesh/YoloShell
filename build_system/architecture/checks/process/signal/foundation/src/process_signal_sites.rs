pub use crate::process_signal_raw_sites::RAW_LIBC_SITES;
use crate::process_signal_unidentified_custody as unidentified;
use yolo_architecture_policy_contracts::process_signal as contract;
use yolo_architecture_syntax::rust_source::RustFunction;

#[derive(Clone, Copy)]
pub struct Site {
    pub path: &'static str,
    pub owner: Option<&'static str>,
    pub function: &'static str,
}

impl Site {
    pub fn matches(&self, path: &str, function: Option<&RustFunction>) -> bool {
        function.is_some_and(|function| {
            self.path == path && function.matches(self.owner, self.function)
        })
    }
}

#[derive(Clone, Copy)]
pub struct NamedSite {
    pub name: &'static str,
    pub site: Site,
}

#[derive(Clone, Copy)]
pub struct ReceiverSite {
    pub receiver: &'static str,
    pub site: Site,
}
pub const ZERO_SIGNAL_SITES: &[Site] = &[
    site(
        "core/crates/runtime/engine/process/identity/src/process.rs",
        None,
        "is_process_alive",
    ),
    site(
        "core/crates/persistence/catalog/src/secure_vfs/secure_vfs_cleanup_error.rs",
        None,
        "process_alive",
    ),
];
pub const PIDFD_SIGNAL_SITE: Site = site(
    contract::PIDFD_KERNEL_PATH,
    Some(contract::PIDFD_OPERATIONS),
    "send",
);
pub const TGKILL_SITE: Site = site(
    contract::BOOTSTRAP_TRACE_PATH,
    None,
    contract::THREAD_BELONGS_TO_PROCESS,
);
pub const AUDIT_SIGNAL_SITE: Site = site(
    contract::MACOS_SIGNAL_PATH,
    None,
    contract::AUDIT_WRAPPER_FUNCTION,
);

pub const CHILD_KILL_SITES: &[ReceiverSite] = &[
    receiver_site(
        contract::SELF_CHILD_RECEIVER,
        contract::DETACHED_CHILD_CUSTODY_PATH,
        Some(contract::DETACHED_CHILD_CUSTODY),
        contract::REQUEST_DIRECT_TERMINATION,
    ),
    receiver_site(
        contract::SELF_CHILD_RECEIVER,
        unidentified::PATH,
        Some(unidentified::OWNER),
        contract::REQUEST_DIRECT_TERMINATION,
    ),
    receiver_site(
        contract::CHILD_RECEIVER,
        contract::BRIDGE_CHILD_PATH,
        Some(contract::SPAWNED_HOST),
        contract::DROP,
    ),
    receiver_site(
        contract::CHILD_RECEIVER,
        contract::SOCKET_ROOT_DIRECT_CHILD_PATH,
        None,
        contract::FORCE_UNREAPED_DIRECT_CHILD,
    ),
];

pub const AUTHORIZED_CALL_SITES: &[Site] = &[
    site(
        contract::SIGNAL_AUTHORITY_PATH,
        Some(contract::PROCESS_SIGNAL_AUTHORITY),
        contract::SIGNAL,
    ),
    site(
        contract::LIVE_CONTROL_PATH,
        Some(contract::LIVE_IDENTITY),
        contract::SIGNAL_FOR_TEST,
    ),
];

pub(crate) const fn named(
    name: &'static str,
    path: &'static str,
    owner: Option<&'static str>,
    function: &'static str,
) -> NamedSite {
    NamedSite {
        name,
        site: site(path, owner, function),
    }
}

pub const fn site(path: &'static str, owner: Option<&'static str>, function: &'static str) -> Site {
    Site {
        path,
        owner,
        function,
    }
}

pub const fn receiver_site(
    receiver: &'static str,
    path: &'static str,
    owner: Option<&'static str>,
    function: &'static str,
) -> ReceiverSite {
    ReceiverSite {
        receiver,
        site: site(path, owner, function),
    }
}
