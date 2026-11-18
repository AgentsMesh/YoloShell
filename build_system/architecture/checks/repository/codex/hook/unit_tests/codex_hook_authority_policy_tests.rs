use yolo_architecture_test_support::Fixture;

const OWNER: &str = "core/crates/runtime/engine/worker/src/codex_hook/runtime/runtime_authority.rs";
const FUTURE: &str = "core/crates/runtime/engine/worker/src/codex_hook/runtime_future.rs";
const AUTHORITY: &str = "core/crates/runtime/engine/worker/src/codex_hook/authority/authority.rs";
const POLICY_PREFIX: &str = "Codex authority";
const PROOF_POLICY: &str = "must pass CodexSnapshotProof through the sealed control publisher";
const SUCCESSION_POLICY: &str = "requires registry-issued CodexSuccessionProof";
const DIRECT_POLICY: &str = "restricted to control publication owners";
const OWNER_POLICY: &str = "restricted to its exact control publication owners";
const SENDER_POLICY: &str = "watch sender may only be observed";
const CONTRACT_POLICY: &str = "control publication contract changed";
const WATCH_MUTATION_COUNT: usize = 4;
const OWNER_SOURCE: &str = "
impl CodexHookRuntime {
fn publish_lifecycle(&self, proof: &CodexSnapshotProof, lifecycle: &LifecycleAuthority) -> Result<()> { if let Some(view) = lifecycle.authority_view() { return self.registration.control().publish_proven(proof, view); } Ok(()) }
fn publish_active(&self, proof: &CodexSnapshotProof, state: NormalizedLifecycle) -> Result<()> { self.registration.control().publish_proven(proof, CodexAuthorityView::Active(state)) }
fn publish_authority_lost(&self) { self.registration.control().publish_terminal_authority_lost(); }
fn publish_storage_unavailable(&self) { self.registration.control().publish_terminal_storage_unavailable(); }
}
impl CodexAuthorityControl {
fn publish_initial(&self, proof: &CodexSnapshotProof, view: CodexAuthorityView) -> Result<()> { self.publish_proven(proof, view) }
fn publish_proven(&self, proof: &CodexSnapshotProof, view: CodexAuthorityView) -> Result<()> {
proof.linearize_publication().context(super::super::constants::CODEX_STORAGE_ERROR)?;
self.authority_tx.send_replace(view); Ok(())
}
fn publish_terminal_authority_lost(&self) {
self.authority_tx.send_if_modified(|view| {
if matches!(view, CodexAuthorityView::AuthorityLost | CodexAuthorityView::Superseded) {
return false; } *view = CodexAuthorityView::AuthorityLost; true
});
}
fn publish_terminal_storage_unavailable(&self) {
self.authority_tx.send_if_modified(|view| {
if matches!(view, CodexAuthorityView::AuthorityLost
| CodexAuthorityView::StorageUnavailable | CodexAuthorityView::Superseded) {
return false; } *view = CodexAuthorityView::StorageUnavailable; true
});
}
fn supersede_locked(&self, lifecycle: &mut LifecycleAuthority, proof: CodexSuccessionProof) {
lifecycle.supersede_authorized(&proof); self.publish_superseded(proof);
}
fn contain_failed_takeover(&self, lifecycle: &mut LifecycleAuthority) {
if lifecycle.prove_snapshot_current().is_ok() || !lifecycle.latch_authority_lost() {
return; } self.publish_terminal_authority_lost();
}
fn publish_superseded(&self, _proof: CodexSuccessionProof) {
self.authority_tx.send_replace(CodexAuthorityView::Superseded);
}
}
";
const PROVEN_FENCE: &str =
    "proof.linearize_publication().context(super::super::constants::CODEX_STORAGE_ERROR)?;
self.authority_tx.send_replace(view);";
const LOST_GUARD: &str = "CodexAuthorityView::AuthorityLost | CodexAuthorityView::Superseded";
const STORAGE_GUARD: &str = "CodexAuthorityView::AuthorityLost
| CodexAuthorityView::StorageUnavailable | CodexAuthorityView::Superseded";

fn messages(fixture: &Fixture) -> Vec<String> {
    crate::inspect(fixture.root())
        .unwrap()
        .into_iter()
        .map(|violation| violation.to_string())
        .filter(|message| message.contains(POLICY_PREFIX))
        .collect()
}

fn write_owner(fixture: &Fixture, source: impl AsRef<[u8]>) {
    fixture.write(OWNER, source);
}

fn edited_messages(old: &str, new: &str) -> Vec<String> {
    let fixture = Fixture::new();
    write_owner(&fixture, OWNER_SOURCE.replace(old, new));
    messages(&fixture)
}

fn contains(messages: &[String], policy: &str) -> bool {
    messages.iter().any(|message| message.contains(policy))
}

#[test]
fn accepts_the_control_owned_contract_and_read_only_subscriptions() {
    let fixture = Fixture::new();
    write_owner(&fixture, OWNER_SOURCE);
    fixture.write(
        AUTHORITY,
        "fn observe(runtime: &Runtime, queue: &Queue) {
let _ = runtime.authority_tx.subscribe(); queue.send(event); queue.send_modify(edit);
queue.send_replace(event); queue.send_if_modified(edit);
// authority_tx.send_modify publish_proven
let _note = \"authority_tx.send_replace\";
}",
    );
    let result = messages(&fixture);
    assert!(result.is_empty(), "{result:#?}");
}

#[test]
fn rejects_every_watch_mutation_outside_the_control_owner() {
    let fixture = Fixture::new();
    write_owner(&fixture, OWNER_SOURCE);
    fixture.write(
        FUTURE,
        "fn bypass(runtime: &Runtime, view: View) {
runtime.authority_tx.send(view);
runtime.authority_tx.send_modify(|_| {});
runtime.authority_tx.send_replace(view);
runtime.authority_tx.send_if_modified(|_| true);
}",
    );
    let result = messages(&fixture);
    assert_eq!(
        result
            .iter()
            .filter(|message| message.contains(DIRECT_POLICY))
            .count(),
        WATCH_MUTATION_COUNT
    );
}

#[test]
fn rejects_unapproved_mutation_and_sender_escape_in_the_owner_file() {
    let source = format!(
        "{OWNER_SOURCE}fn bypass(control: &Control, view: View) {{
control.authority_tx.send(view);
let escaped = &control.authority_tx; escaped.send_replace(view);
}}"
    );
    let fixture = Fixture::new();
    write_owner(&fixture, source);
    let result = messages(&fixture);
    assert!(contains(&result, OWNER_POLICY));
    assert!(contains(&result, SENDER_POLICY));
}

#[test]
fn rejects_proof_free_or_non_contiguous_recoverable_publication() {
    assert!(contains(
        &edited_messages("proof: &CodexSnapshotProof", "proof: CodexSnapshotProof"),
        CONTRACT_POLICY
    ));
    let replacements = [
        "self.authority_tx.send_replace(view); proof.linearize_publication().context(super::super::constants::CODEX_STORAGE_ERROR)?;",
        "proof.linearize_publication().context(super::super::constants::CODEX_STORAGE_ERROR)?; wait_for_io(); self.authority_tx.send_replace(view);",
    ];
    for replacement in replacements {
        assert!(contains(
            &edited_messages(PROVEN_FENCE, replacement),
            CONTRACT_POLICY
        ));
    }
    let fixture = Fixture::new();
    write_owner(&fixture, OWNER_SOURCE);
    fixture.write(
        FUTURE,
        "fn bypass(control: &Control, proof: &CodexSnapshotProof, view: View) {
control.publish_proven(proof, view);
}",
    );
    assert!(contains(&messages(&fixture), PROOF_POLICY));
}

#[test]
fn rejects_terminal_publication_that_can_overwrite_a_terminal_view() {
    let changes = [
        (LOST_GUARD, "CodexAuthorityView::AuthorityLost"),
        (STORAGE_GUARD, "CodexAuthorityView::StorageUnavailable"),
        (
            "self.publish_terminal_authority_lost();\n}",
            "self.publish_terminal_storage_unavailable();\n}",
        ),
    ];
    for (original, replacement) in changes {
        assert!(contains(
            &edited_messages(original, replacement),
            CONTRACT_POLICY
        ));
    }
}

#[test]
fn rejects_superseded_publication_without_registry_proof() {
    let borrowed = edited_messages(
        "_proof: CodexSuccessionProof",
        "_proof: &CodexSuccessionProof",
    );
    assert!(contains(&borrowed, CONTRACT_POLICY));
    let fixture = Fixture::new();
    write_owner(&fixture, OWNER_SOURCE);
    fixture.write(
        FUTURE,
        "fn bypass(control: &Control, proof: CodexSuccessionProof) {
control.publish_superseded(proof);
}",
    );
    assert!(contains(&messages(&fixture), SUCCESSION_POLICY));
}
