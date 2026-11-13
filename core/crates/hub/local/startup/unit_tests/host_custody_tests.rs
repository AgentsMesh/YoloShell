use crate::host::require_custody_available;

#[test]
fn custody_requirement_reports_the_actionable_remedy() {
    assert!(
        require_custody_available(false)
            .unwrap_err()
            .to_string()
            .contains("delegated cgroup v2 custody")
    );
    assert!(require_custody_available(true).is_ok());
}
