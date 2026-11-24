use super::support::{ALPHA, ALPHA_BUILD, assert_reports, boundaries, files, messages};

#[test]
fn rejects_a_package_mixing_primary_roles() {
    let report = messages(files(
        &boundaries(
            &["//core/crates/hub/alpha:lib"],
            &["//core/crates/hub/alpha:lib"],
            &[],
            &[],
        ),
        &[(ALPHA_BUILD, ALPHA)],
    ));

    assert_reports(&report, "mixes public, client, and runtime roles");
}

#[test]
fn rejects_a_duplicate_role_label() {
    let report = messages(files(
        &boundaries(
            &[
                "//core/crates/hub/alpha:lib",
                "//core/crates/hub/alpha:lib",
            ],
            &[],
            &[],
            &[],
        ),
        &[(ALPHA_BUILD, ALPHA)],
    ));

    assert_reports(&report, "classified more than once");
}

#[test]
fn admits_an_entry_alongside_a_client_role() {
    let report = messages(files(
        &boundaries(
            &[],
            &["//core/crates/hub/alpha:lib"],
            &["//core/crates/hub/alpha:lib"],
            &[],
        ),
        &[(ALPHA_BUILD, ALPHA)],
    ));

    assert!(report.is_empty(), "{report:?}");
}

#[test]
fn rejects_a_missing_role_list() {
    let text = "PUBLIC_COMPONENTS = []\nCLIENT_COMPONENTS = []\nHUB_ENTRY_COMPONENTS = []\n";
    let report = messages(files(text, &[]));

    assert_reports(&report, "Hub role lists");
}

#[test]
fn rejects_an_unresolved_topology_name() {
    let text = "PUBLIC_COMPONENTS = [MISSING_HUB_LABEL]\nCLIENT_COMPONENTS = []\nHUB_ENTRY_COMPONENTS = []\nRUNTIME_COMPONENTS = []\n";
    let report = messages(files(text, &[]));

    assert_reports(&report, "topology manifest");
}
