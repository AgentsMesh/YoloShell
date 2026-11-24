use crate::process::captured_tree::gone::{self, ReportedProducersGoneRetirement};
use crate::process::{CapturedProcessTree, ProcessIdentity, ReportedProducersGone};

const FOREIGN_COMMIT_DETAIL: &str = "foreign reported retirement authority unexpectedly committed";
const MISMATCH_DETAIL: &str = "belongs to another process closure";
const MASTER_RELEASE_DETAIL: &str = "non-PTY retirement unexpectedly required master release";

#[test]
fn reported_retirement_rejects_a_foreign_preparation_token() {
    let mut owner = reported();
    let foreign = reported();
    let owner_ready = owner.prepare_retirement().unwrap();

    let failure = match foreign.into_retirement(owner_ready) {
        Err(failure) => failure,
        Ok(_) => panic!("{FOREIGN_COMMIT_DETAIL}"),
    };
    assert!(failure.error().to_string().contains(MISMATCH_DETAIL));
    let mut foreign = failure.into_state();
    assert!(!foreign.session_is_retired());

    let foreign_ready = foreign.prepare_retirement().unwrap();
    let retired = match foreign.into_retirement(foreign_ready).unwrap() {
        ReportedProducersGoneRetirement::Direct(ready) => ready.commit(),
        ReportedProducersGoneRetirement::MasterRelease(_) => panic!("{MASTER_RELEASE_DETAIL}"),
    };
    assert!(retired.session_is_retired());
}

fn reported() -> ReportedProducersGone {
    let root = ProcessIdentity::capture(std::process::id()).unwrap();
    let tree = CapturedProcessTree::capture(root).unwrap();
    let tracked = gone::complete(tree);
    tracked.report(|_| Ok(())).unwrap().0
}
