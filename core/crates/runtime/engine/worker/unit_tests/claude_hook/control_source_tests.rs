// Include as a child module of worker::tests to reuse its retained Worker fixture.
use super::*;
#[path = "../../test_support/unit/claude_hook/control_source_fixture.rs"]
mod support;
use std::cell::Cell;
use support::NativeSource;
use yolo_agent_control_contract::NativeControlWitness;

#[test]
fn native_claude_capture_and_exact_witness_execute_under_durable_source_guard() {
    let source = NativeSource::new();
    let (binding, witness) = source
        .with(None, |binding, witness, verify| {
            verify()?;
            Ok((binding.clone(), witness.clone()))
        })
        .unwrap();
    let returned = source
        .with(Some(&witness), |current, observed, verify| {
            verify()?;
            assert_eq!(current, &binding);
            assert_eq!(observed, &witness);
            assert!(
                source
                    .fixture
                    .state
                    .agent_generation_gate
                    .try_lock()
                    .is_err()
            );
            assert!(source.fixture.state.mutation_gate.try_lock().is_err());
            assert!(
                crate::paths::durable_mutation::SessionDurableMutationGuard::acquire(
                    &source.fixture.state.paths,
                    source.fixture.state.namespace.lease(),
                    source.fixture.terminal_id(),
                )
                .is_err()
            );
            Ok(true)
        })
        .unwrap();
    assert!(returned);
}

#[test]
fn native_claude_wrong_witness_or_new_source_revision_never_calls_effect() {
    let mut source = NativeSource::new();
    let witness = source
        .with(None, |_, witness, _| Ok(witness.clone()))
        .unwrap();
    let mut changed = witness.fields().clone();
    changed.worker_id = Uuid::new_v4();
    let changed = NativeControlWitness::new(changed).unwrap();
    let called = Cell::new(false);
    assert!(
        source
            .with(Some(&changed), |_, _, _| {
                called.set(true);
                Ok(())
            })
            .is_err()
    );
    assert!(!called.get());
    source.change_activity();
    assert!(
        source
            .with(Some(&witness), |_, _, _| {
                called.set(true);
                Ok(())
            })
            .is_err()
    );
    assert!(!called.get());
    let current = source
        .with(None, |_, witness, _| Ok(witness.clone()))
        .unwrap();
    assert_ne!(
        current.fields().authority_digest,
        witness.fields().authority_digest
    );
}

#[test]
fn native_claude_revocation_and_projection_corruption_never_call_effect() {
    for revoke in [false, true] {
        let source = NativeSource::new();
        let witness = source
            .with(None, |_, witness, _| Ok(witness.clone()))
            .unwrap();
        if revoke {
            source.revoke();
        } else {
            let mut binding = source.record.recovery_binding(true);
            binding.state = crate::agent::AgentState::Running;
            crate::agent::AgentBindingStore::new(
                source
                    .fixture
                    .state
                    .paths
                    .agent_binding(source.fixture.terminal_id()),
            )
            .save(&binding)
            .unwrap();
        }
        let called = Cell::new(false);
        assert!(
            source
                .with(Some(&witness), |_, _, _| {
                    called.set(true);
                    Ok(())
                })
                .is_err()
        );
        assert!(!called.get());
    }
}

#[derive(Clone, Copy)]
enum SourceFile {
    Binding,
    Authority,
    Runtime,
}

#[test]
fn post_intent_claude_verification_rejects_same_content_source_replacement_without_relocking() {
    for file in [
        SourceFile::Binding,
        SourceFile::Authority,
        SourceFile::Runtime,
    ] {
        let source = NativeSource::new();
        let state = &source.fixture.state;
        let binding_path = state.paths.agent_binding(source.fixture.terminal_id());
        let path = match file {
            SourceFile::Binding => binding_path.clone(),
            SourceFile::Authority => crate::agent::claude::generation::native::ClaudeNativeGenerationStore::from_binding_path(&binding_path)
                .unwrap().path().to_path_buf(),
            SourceFile::Runtime => state.paths.runtime_record(source.fixture.terminal_id()),
        };
        let called = Cell::new(false);
        assert!(
            source
                .with(None, |_, _, verify| {
                    verify()?;
                    let value: serde_json::Value = state.namespace.lease().read_json(&path)?;
                    state.namespace.lease().write_json(&path, &value)?;
                    verify()?;
                    called.set(true);
                    Ok(())
                })
                .is_err()
        );
        assert!(!called.get());
    }
}
