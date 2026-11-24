use yolo_architecture_test_support::Fixture;

const CONTROL: &str = "core/crates/runtime/engine/worker/src/codex_hook/authority/authority_control.rs";
const TAKEOVER: &str = "core/crates/runtime/engine/worker/src/codex_hook/runtime/runtime_takeover.rs";
const CHANNEL_POLICY: &str = "watch channel must have one fail-closed control initialization";
const TAKEOVER_POLICY: &str = "registry install must immediately supersede its predecessor";
const CONTROL_SOURCE: &str = "impl CodexAuthorityControl {
fn new(lifecycle: LifecycleAuthority) -> Arc<Self> {
let (authority_tx, _) = tokio::sync::watch::channel(CodexAuthorityView::StorageUnavailable);
Arc::new(Self { lifecycle: Mutex::new(lifecycle), authority_tx })
}
}";
const TAKEOVER_SOURCE: &str = "fn finish(
takeover: CodexAuthorityTakeover<'_>, predecessor: Option<Predecessor>
) -> Result<()> {
let succession = takeover.install(&runtime.registration);
if let (Some((predecessor, lifecycle)), Some(proof)) = (predecessor, succession) {
predecessor.control().supersede_locked(lifecycle, proof);
}
Ok(())
}";
const TAKEOVER_SEQUENCE: &str = "let succession = takeover.install(&runtime.registration);
if let (Some((predecessor, lifecycle)), Some(proof)) = (predecessor, succession) {
predecessor.control().supersede_locked(lifecycle, proof);
}";

fn messages(fixture: &Fixture, policy: &str) -> Vec<String> {
    crate::inspect(fixture.root())
        .unwrap()
        .into_iter()
        .map(|violation| violation.to_string())
        .filter(|message| message.contains(policy))
        .collect()
}

fn channel_messages(source: &str) -> Vec<String> {
    let fixture = Fixture::new();
    fixture.write(CONTROL, source);
    messages(&fixture, CHANNEL_POLICY)
}

fn takeover_messages(source: &str) -> Vec<String> {
    let fixture = Fixture::new();
    fixture.write(TAKEOVER, source);
    messages(&fixture, TAKEOVER_POLICY)
}

#[test]
fn accepts_the_unique_fail_closed_watch_channel() {
    assert!(channel_messages(CONTROL_SOURCE).is_empty());
}

#[test]
fn rejects_recoverable_or_renamed_watch_channel_initialization() {
    let bypasses = [
        CONTROL_SOURCE.replace("StorageUnavailable", "Active(state)"),
        CONTROL_SOURCE.replace("StorageUnavailable", "Exhausted(state)"),
        CONTROL_SOURCE.replace("authority_tx, _", "replacement_tx, _"),
    ];
    for bypass in bypasses {
        assert!(!channel_messages(&bypass).is_empty(), "{bypass}");
    }
}

#[test]
fn rejects_a_second_or_moved_watch_channel() {
    let duplicate = CONTROL_SOURCE.replace(
        "Arc::new(Self",
        "let _ = tokio::sync::watch::channel(CodexAuthorityView::StorageUnavailable); Arc::new(Self",
    );
    assert!(!channel_messages(&duplicate).is_empty());
    let fixture = Fixture::new();
    fixture.write(CONTROL, CONTROL_SOURCE);
    fixture.write(
        "core/crates/runtime/engine/worker/src/codex_hook/future.rs",
        "fn bypass() { let _ = watch::channel(CodexAuthorityView::Active(state)); }",
    );
    assert!(!messages(&fixture, CHANNEL_POLICY).is_empty());
}

#[test]
fn accepts_the_contiguous_registry_install_and_supersession() {
    assert!(takeover_messages(TAKEOVER_SOURCE).is_empty());
}

#[test]
fn does_not_treat_the_supersession_method_definition_as_a_call() {
    let fixture = Fixture::new();
    fixture.write(TAKEOVER, TAKEOVER_SOURCE);
    fixture.write(
        "core/crates/runtime/engine/worker/src/codex_hook/runtime/runtime_authority.rs",
        "impl CodexAuthorityControl {
fn supersede_locked(&self, lifecycle: &mut LifecycleAuthority, proof: CodexSuccessionProof) {}
}",
    );
    assert!(messages(&fixture, TAKEOVER_POLICY).is_empty());
}

#[test]
fn rejects_work_inserted_after_registry_replacement() {
    let insertions = [
        "callback();",
        "wait_for_io().await;",
        "fallible()?;",
        "assert!(ready);",
        "let _allocation = Arc::new(value);",
    ];
    for insertion in insertions {
        let replacement = TAKEOVER_SEQUENCE.replace(
            "if let (Some((predecessor, lifecycle))",
            &format!("{insertion} if let (Some((predecessor, lifecycle))"),
        );
        let bypass = TAKEOVER_SOURCE.replace(TAKEOVER_SEQUENCE, &replacement);
        assert!(!takeover_messages(&bypass).is_empty(), "{insertion}");
    }
}

#[test]
fn rejects_moved_or_missing_predecessor_supersession() {
    let missing = TAKEOVER_SOURCE.replace(
        "predecessor.control().supersede_locked(lifecycle, proof);",
        "observe(predecessor, proof);",
    );
    assert!(!takeover_messages(&missing).is_empty());
    let fixture = Fixture::new();
    fixture.write(TAKEOVER, TAKEOVER_SOURCE);
    fixture.write(
        "core/crates/runtime/engine/worker/src/codex_hook/future.rs",
        "fn bypass() { predecessor.control().supersede_locked(lifecycle, proof); }",
    );
    assert!(!messages(&fixture, TAKEOVER_POLICY).is_empty());
}

#[test]
fn similarly_named_tokens_do_not_count_as_watch_or_takeover_operations() {
    for text in [
        "channel",
        "install",
        yolo_architecture_policy_contracts::constants::CODEX_AUTHORITY_SUPERSEDE_LOCKED,
        "watch.channel()",
        "watch + channel",
    ] {
        let fixture = Fixture::new();
        fixture.write(CONTROL, CONTROL_SOURCE);
        fixture.write(TAKEOVER, TAKEOVER_SOURCE);
        fixture.write(
            "core/crates/runtime/engine/worker/src/codex_hook/future.rs",
            text,
        );
        assert!(messages(&fixture, CHANNEL_POLICY).is_empty(), "{text}");
        assert!(messages(&fixture, TAKEOVER_POLICY).is_empty(), "{text}");
    }
}

#[test]
fn takeover_must_install_exactly_once_even_when_the_original_sequence_survives() {
    let duplicate =
        TAKEOVER_SOURCE.replace("Ok(())", "takeover.install(&runtime.registration); Ok(())");
    assert!(!takeover_messages(&duplicate).is_empty());
}
