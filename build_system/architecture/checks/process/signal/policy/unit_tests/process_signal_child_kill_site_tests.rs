use super::fixture::Fixture;
use super::process_signal_test_support::messages;

const DETACHED_CUSTODY_PATH: &str =
    "core/crates/runtime/engine/process/src/detached_child_custody.rs";
const UNIDENTIFIED_CUSTODY_PATH: &str =
    "core/crates/runtime/engine/process/src/unidentified_pty_child_custody.rs";
const BRIDGE_CHILD_PATH: &str = "core/crates/application/host/cli/src/bridge_child.rs";
const KILL_POLICY: &str = "direct child termination is outside its exact process custody owner";

#[test]
fn exact_custody_sites_reject_every_alternate_receiver() {
    let fixture = Fixture::new();
    fixture.write(
        DETACHED_CUSTODY_PATH,
        r#"
impl DetachedChildCustody {
fn request_direct_termination(&mut self) {
    other.kill();
    self.child.kill();
    child.child.kill();
}
}
"#,
    );
    fixture.write(
        UNIDENTIFIED_CUSTODY_PATH,
        r#"
impl UnidentifiedPtyChildCustody {
fn request_direct_termination(&mut self) {
    other.kill();
    self.child.kill();
    child.child.kill();
}
}
"#,
    );
    fixture.write(
        BRIDGE_CHILD_PATH,
        r#"
impl Drop for SpawnedHost {
    fn drop(&mut self) {
        other.kill();
        self.child.kill();
        self.kill();
    }
}
"#,
    );
    assert_eq!(
        messages(&fixture)
            .iter()
            .filter(|message| message.contains(KILL_POLICY))
            .count(),
        8
    );
}

#[test]
fn exact_custody_requires_rooted_std_child_ufcs_and_argument() {
    let fixture = Fixture::new();
    fixture.write(
        DETACHED_CUSTODY_PATH,
        r#"
impl DetachedChildCustody {
fn request_direct_termination(&mut self) {
    Child::kill(child);
    ::std::process::Child::kill(&mut other);
}
}
"#,
    );
    fixture.write(
        UNIDENTIFIED_CUSTODY_PATH,
        r#"
impl UnidentifiedPtyChildCustody {
fn request_direct_termination(&mut self) {
    self.child.kill();
    self.other.kill();
}
}
"#,
    );
    fixture.write(
        BRIDGE_CHILD_PATH,
        r#"
impl Drop for SpawnedHost {
    fn drop(&mut self) {
        child.kill();
        ::std::process::Child::kill(&mut other);
    }
}
"#,
    );
    assert_eq!(
        messages(&fixture)
            .iter()
            .filter(|message| message.contains(KILL_POLICY))
            .count(),
        5
    );
}
