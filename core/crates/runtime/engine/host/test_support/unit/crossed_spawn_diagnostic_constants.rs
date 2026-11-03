use crate::host::tests::*;
pub(super) const READ_LABEL: &str = "crossed-spawn diagnostic evidence";
pub(super) const BASELINE_LOCK: &str = "crossed-spawn diagnostic cannot lock creation ledger";
pub(super) const BASELINE_FINGERPRINT: &str =
    "crossed-spawn diagnostic cannot fingerprint the original request";
pub(super) const BASELINE_MISSING: &str =
    "crossed-spawn diagnostic has no binding for the original request";
pub(super) const BASELINE_AMBIGUOUS: &str =
    "crossed-spawn diagnostic found more than one binding for the original request";
pub(super) const BASELINE_MISMATCH: &str =
    "crossed-spawn diagnostic request and catalog session do not share one binding";
