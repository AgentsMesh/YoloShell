use super::fixtures::Fixture;
use std::{fs, path::Path};
use yolo_agent_contract::AgentKind;

#[test]
fn another_agent_creating_a_sibling_file_does_not_change_this_native_evidence() {
    let fixture = Fixture::new(AgentKind::Codex);
    let mut proof = yolo_codex::CodexAdapter
        .inspect_authoritative_evidence_with_hook(
            &fixture.path,
            Path::new(&fixture.binding.cwd),
            Path::new(&fixture.binding.data_home),
            || {},
        )
        .unwrap()
        .unwrap();
    assert!(
        proof
            .with_transcript_snapshot(|_, _| {
                fs::write(
                    fixture.path.parent().unwrap().join("another-agent.jsonl"),
                    b"{}\n",
                )?;
                Ok(())
            })
            .unwrap()
            .is_some()
    );
}

#[test]
fn claude_in_place_session_rewrite_between_inspection_and_read_is_rejected() {
    let fixture = Fixture::new(AgentKind::ClaudeCode);
    let mut proof = yolo_claude::ClaudeAdapter
        .inspect_authoritative_evidence_with_hook(
            &fixture.path,
            Path::new(&fixture.binding.cwd),
            Path::new(&fixture.binding.data_home),
            || {},
        )
        .unwrap()
        .unwrap();
    let text = fs::read_to_string(&fixture.path).unwrap();
    let changed = text.replace(
        &fixture.binding.provider_session_id,
        "01a1196f-84d9-7493-82bc-8468d635823e",
    );
    fs::write(&fixture.path, changed).unwrap();
    assert!(
        proof
            .with_transcript_snapshot::<()>(|_, _| panic!("rebound contents must not be read"))
            .unwrap()
            .is_none()
    );
}

#[test]
fn claude_allows_append_but_rejects_path_replacement() {
    for replace in [false, true] {
        let fixture = Fixture::new(AgentKind::ClaudeCode);
        let mut proof = yolo_claude::ClaudeAdapter
            .inspect_authoritative_evidence_with_hook(
                &fixture.path,
                Path::new(&fixture.binding.cwd),
                Path::new(&fixture.binding.data_home),
                || {},
            )
            .unwrap()
            .unwrap();
        assert_eq!(
            proof
                .with_transcript_snapshot(|_, _| {
                    if replace {
                        fs::rename(&fixture.path, fixture.directory.path().join("retired"))
                            .unwrap();
                    } else {
                        fixture.message("concurrent append");
                    }
                    Ok(())
                })
                .unwrap()
                .is_none(),
            replace
        );
    }
}
