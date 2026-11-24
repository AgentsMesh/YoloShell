use yolo_architecture_test_support::Fixture;

const OWNER_PATH: &str = "core/crates/runtime/engine/process/src/captured/captured_tree.rs";
const CALLER_PATH: &str = "core/crates/runtime/engine/process/src/captured/tree/gone/captured_tree_gone_retirement.rs";
const BOOTSTRAP_CALLER_PATH: &str =
    "core/crates/runtime/engine/process/src/captured/tree/captured_tree_bootstrap_failure.rs";
const FUTURE_PATH: &str = "core/crates/runtime/engine/process/src/future_retirement.rs";
const FUTURE_CONSTANTS_PATH: &str =
    "core/crates/runtime/engine/process/src/future_retirement_constants.rs";
const OWNERSHIP_TEXT: &str = "typed gone-retirement commit boundary";
const DEFINITION_TEXT: &str = "retirement must remain private and unique";
const CALLER_TEXT: &str = "retirement commits must each retire exactly once";

fn messages(fixture: &Fixture) -> Vec<String> {
    crate::inspect(fixture.root())
        .unwrap()
        .into_iter()
        .map(|violation| violation.to_string())
        .collect()
}

fn write_contract(fixture: &Fixture) {
    fixture.write(
        OWNER_PATH,
        "impl CapturedProcessTree {\nfn retire_session(&mut self) {}\n#[cfg(test)] fn retire_session_for_test(&mut self) { self.retire_session(); }\n}\n",
    );
    fixture.write(
        CALLER_PATH,
        "impl DirectRetirementReady { fn commit(mut self) { self.0.tree.retire_session(); } }\nimpl MasterReleasePendingProducersGone { fn commit_retirement_after_master_release(self) { completion.tree.retire_session(); } }\n",
    );
    fixture.write(
        BOOTSTRAP_CALLER_PATH,
        "impl ExplicitBootstrapFailureRetirementReady { fn commit(self) { self.tree.retire_session(); } }\n",
    );
}

#[test]
fn accepts_the_private_typed_retirement_boundary() {
    let fixture = Fixture::new();
    write_contract(&fixture);
    assert!(messages(&fixture).is_empty());
}

#[test]
fn rejects_public_or_duplicate_retirement_definitions() {
    let public = Fixture::new();
    write_contract(&public);
    public.write(
        OWNER_PATH,
        "impl CapturedProcessTree { pub(crate) fn retire_session(&mut self) {} }\n",
    );
    let public_messages = messages(&public);
    assert!(
        public_messages
            .iter()
            .any(|message| message.contains(OWNERSHIP_TEXT))
    );
    assert!(
        public_messages
            .iter()
            .any(|message| message.contains(DEFINITION_TEXT))
    );

    let duplicate = Fixture::new();
    write_contract(&duplicate);
    duplicate.write(
        OWNER_PATH,
        "impl CapturedProcessTree { fn retire_session(&mut self) {} fn duplicate(&mut self) { self.retire_session(); } }\n",
    );
    assert!(
        messages(&duplicate)
            .iter()
            .any(|message| message.contains(OWNERSHIP_TEXT))
    );
}

#[test]
fn rejects_moved_missing_or_duplicated_commit_calls() {
    let moved = Fixture::new();
    write_contract(&moved);
    moved.write(
        FUTURE_PATH,
        "fn bypass(tree: &mut Tree) { tree.retire_session(); }\n",
    );
    assert!(
        messages(&moved)
            .iter()
            .any(|message| message.contains(OWNERSHIP_TEXT))
    );

    let missing = Fixture::new();
    write_contract(&missing);
    missing.write(BOOTSTRAP_CALLER_PATH, "fn no_commit() {}\n");
    assert!(
        messages(&missing)
            .iter()
            .any(|message| message.contains(CALLER_TEXT))
    );

    let duplicated = Fixture::new();
    write_contract(&duplicated);
    duplicated.write(
        BOOTSTRAP_CALLER_PATH,
        "impl ExplicitBootstrapFailureRetirementReady { fn commit(self) { self.tree.retire_session(); self.tree.retire_session(); } }\n",
    );
    assert!(
        messages(&duplicated)
            .iter()
            .any(|message| message.contains(CALLER_TEXT))
    );
}

#[test]
fn comments_strings_and_test_sources_do_not_mint_retirement() {
    let fixture = Fixture::new();
    write_contract(&fixture);
    fixture.write(
        FUTURE_CONSTANTS_PATH,
        "// retire_session()\nconst NOTE: &str = \"retire_session\";\n",
    );
    fixture.write(
        FUTURE_PATH,
        "#[cfg(test)] fn fixture(tree: &mut Tree) { tree.retire_session(); }\n",
    );
    fixture.write(
        "core/crates/runtime/engine/process/src/future_retirement_tests.rs",
        "fn fixture(tree: &mut Tree) { tree.retire_session(); }\n",
    );
    let actual = messages(&fixture);
    assert!(actual.is_empty(), "{actual:#?}");
}
