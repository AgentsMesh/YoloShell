pub use crate::process_signal_constructor_sites::CONSTRUCTOR_SITES;
use crate::process_signal_sites::{Site, site};
use yolo_architecture_policy_contracts::process_signal as contract;

#[derive(Clone, Copy)]
pub struct QualifiedSite {
    pub qualifier: &'static str,
    pub name: &'static str,
    pub site: Site,
}

pub const MEMBER_TEST_CONSTRUCTORS: &[QualifiedSite] = &[
    qualified(
        contract::PROCESS_GROUP_IDENTITY,
        contract::VERIFIED,
        contract::MEMBER_SIGNAL_TEST_SUPPORT_PATH,
        Some("MemberProcess"),
        "group",
    ),
    qualified(
        contract::PROCESS_GROUP_IDENTITY,
        contract::VERIFIED,
        contract::MEMBER_SIGNAL_UNIT_TEST_PATH,
        None,
        "signal_validation_rejects_probe_invalid_and_force_signals_before_inspection",
    ),
    qualified(
        contract::PROCESS_GROUP_IDENTITY,
        contract::VERIFIED,
        contract::MEMBER_SIGNAL_UNIT_TEST_PATH,
        None,
        "unbound_process_locators_cannot_be_used_as_member_authority",
    ),
];

pub const AUTHORITY_SIGNAL_SITES: &[Site] = &[
    site(
        contract::SIGNAL_AUTHORITY_PATH,
        Some(contract::DESTRUCTIVE_AUTHORITY),
        contract::SIGNAL,
    ),
    site(
        contract::SIGNAL_AUTHORITY_PATH,
        Some(contract::FOREGROUND_AUTHORITY),
        contract::SIGNAL,
    ),
    site(
        contract::SIGNAL_AUTHORITY_PATH,
        None,
        contract::REQUEST_WAITABLE_ROOT_GRACEFUL_TERMINATION,
    ),
    site(
        contract::SIGNAL_AUTHORITY_PATH,
        None,
        contract::SIGNAL_EXACT_PROCESS,
    ),
    site(contract::CAPTURED_TARGETS_PATH, None, contract::SIGNAL),
    site(contract::INVENTORY_PATH, None, "signal_process_group"),
    site(
        contract::INVENTORY_MEMBER_SIGNAL_PATH,
        None,
        contract::SIGNAL_PROCESS_GROUP_MEMBER,
    ),
    site(
        contract::LIVE_CONTROL_PATH,
        Some(contract::LIVE_IDENTITY),
        contract::SIGNAL_AUTHORIZED,
    ),
];

pub const REQUIRED_AUTHORITY_FUNCTIONS: &[Site] = &[
    site(
        contract::LIVE_CONTROL_PATH,
        Some(contract::LIVE_IDENTITY),
        contract::SIGNAL_AUTHORIZED,
    ),
    site(
        contract::SIGNAL_AUTHORITY_PATH,
        Some(contract::DESTRUCTIVE_AUTHORITY),
        contract::SIGNAL,
    ),
    site(
        contract::SIGNAL_AUTHORITY_PATH,
        Some(contract::FOREGROUND_AUTHORITY),
        contract::VERIFIED,
    ),
    site(
        contract::SIGNAL_AUTHORITY_PATH,
        Some(contract::FOREGROUND_AUTHORITY),
        contract::SIGNAL,
    ),
    site(
        contract::SIGNAL_AUTHORITY_PATH,
        Some(contract::PROCESS_SIGNAL_AUTHORITY),
        contract::NEW,
    ),
    site(
        contract::SIGNAL_AUTHORITY_PATH,
        Some(contract::PROCESS_SIGNAL_AUTHORITY),
        contract::SIGNAL,
    ),
    site(
        contract::SIGNAL_AUTHORITY_PATH,
        None,
        contract::SIGNAL_EXACT_PROCESS,
    ),
    site(
        contract::INVENTORY_PATH,
        Some(contract::PROCESS_GROUP_IDENTITY),
        contract::VERIFIED,
    ),
];

pub const TEST_BRIDGES: &[Site] = &[
    site(
        contract::LIVE_CONTROL_PATH,
        Some(contract::LIVE_IDENTITY),
        contract::SIGNAL_FOR_TEST,
    ),
    site(
        contract::LIVE_CONTROL_PATH,
        Some(contract::LIVE_IDENTITY),
        contract::SIGNAL,
    ),
    site(
        contract::SIGNAL_AUTHORITY_PATH,
        Some(contract::DESTRUCTIVE_ACCESS),
        contract::AUTHORIZE_FOR_TEST,
    ),
    site(
        contract::SIGNAL_AUTHORITY_PATH,
        Some(contract::PROCESS_SIGNAL_AUTHORITY),
        contract::FOR_TEST,
    ),
    site(
        contract::INVENTORY_PATH,
        Some(contract::PROCESS_IDENTITY),
        contract::SIGNAL,
    ),
    site(
        contract::CAPTURED_TARGETS_PATH,
        None,
        contract::SIGNAL_FOR_TEST,
    ),
];

pub(crate) const fn qualified(
    qualifier: &'static str,
    name: &'static str,
    path: &'static str,
    owner: Option<&'static str>,
    function: &'static str,
) -> QualifiedSite {
    QualifiedSite {
        qualifier,
        name,
        site: site(path, owner, function),
    }
}

pub(crate) const fn authority_constructor(
    owner: Option<&'static str>,
    function: &'static str,
) -> QualifiedSite {
    qualified(
        contract::PROCESS_SIGNAL_AUTHORITY,
        contract::NEW,
        contract::SIGNAL_AUTHORITY_PATH,
        owner,
        function,
    )
}

pub(crate) const fn retained_operation(
    operation: &'static str,
    path: &'static str,
    owner: &'static str,
    function: &'static str,
) -> QualifiedSite {
    qualified(
        contract::RETAINED_EXACT_PROCESS,
        operation,
        path,
        Some(owner),
        function,
    )
}
