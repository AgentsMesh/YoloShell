use crate::process_signal_guardian_constants as guardian;
use yolo_architecture_policy_contracts::process_signal as contract;
use yolo_architecture_policy_contracts::process_signal_retained as retained;

pub struct TypeSite {
    path: &'static str,
    name: &'static str,
}

const TYPE_SITES: &[TypeSite] = &[
    type_site(
        contract::EXACT_TERMINATION_PATH,
        contract::RETAINED_EXACT_PROCESS,
    ),
    type_site(
        retained::PROCESS_MODULE_PATH,
        contract::RETAINED_EXACT_PROCESS,
    ),
    type_site(guardian::PATH, retained::NAMESPACE_LOSS_CONTAINMENT_OWNER),
    type_site(guardian::PATH, guardian::TARGET_OWNER),
];

pub fn allows_type(path: &str, name: &str) -> bool {
    TYPE_SITES
        .iter()
        .any(|site| site.path == path && site.name == name)
}

pub fn requires_exact_type_site(name: &str) -> bool {
    TYPE_SITES.iter().any(|site| site.name == name)
}

pub fn requires_exact_type_path(path: &str) -> bool {
    TYPE_SITES.iter().any(|site| site.path == path)
}

const fn type_site(path: &'static str, name: &'static str) -> TypeSite {
    TypeSite { path, name }
}
