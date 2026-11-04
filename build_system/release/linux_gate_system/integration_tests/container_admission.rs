use super::*;
use std::fs;
use std::os::unix::fs::PermissionsExt;
#[path = "../test_support/system_engine.rs"]
mod cli;
use cli::DockerFixture;

const EVIDENCE_NAME: &str = "source-evidence";
const EVIDENCE_BYTES: &[u8] = b"preserve until exact owned container is absent";
const OTHER_NAME: &str = "/foreign-container";
const OTHER_IMAGE: &str = "foreign-image";
const OTHER_COMMAND: &str = "foreign-command";
const OTHER_TEMP: &str = "/foreign-tmp";
const MODE_MASK: u32 = 0o777;
const NO_ACCESS: u32 = 0;
enum Change {
    Name,
    Image,
    Command,
    ForeignTmpfs,
    ValidTmpfs,
}

fn document(observed: &Inspection) -> serde_json::Value {
    let mounts = observed
        .mounts
        .iter()
        .map(|mount| {
            serde_json::json!({
                "Type": mount.kind, "Source": mount.source,
                "Destination": mount.destination, "RW": mount.writable,
            })
        })
        .collect::<Vec<_>>();
    serde_json::json!([{
        "Id": observed.id, "Name": observed.name,
        "Config": {"Image": observed.config.image, "Cmd": observed.config.cmd,
            "Labels": observed.config.labels}, "Mounts": mounts,
    }])
}

#[test]
fn daemon_observed_admission_mismatch_retains_snapshot_while_valid_tmpfs_settles() {
    for change in [
        Change::Name,
        Change::Image,
        Change::Command,
        Change::ForeignTmpfs,
        Change::ValidTmpfs,
    ] {
        let snapshot =
            GateSnapshot::fixture_with_source(&[(EVIDENCE_NAME, EVIDENCE_BYTES)]).unwrap();
        let root = snapshot.root_path();
        let evidence = snapshot.source().display_path().join(EVIDENCE_NAME);
        let mode = fs::metadata(&evidence).unwrap().permissions().mode() & MODE_MASK;
        let docker = DockerFixture::new();
        let engine = docker.engine(&snapshot);
        let expected = contract(&snapshot);
        let observed = inspection(&expected);
        docker.write(
            "filter",
            format!("name=^/{}$", expected.command.name).as_bytes(),
        );
        docker.write("list", b"");
        let owner = Owner::prepare(&snapshot, &engine, expected, None).unwrap();
        let mut value = document(&observed);
        let settles = matches!(change, Change::ValidTmpfs);
        match change {
            Change::Name => value[0]["Name"] = OTHER_NAME.into(),
            Change::Image => value[0]["Config"]["Image"] = OTHER_IMAGE.into(),
            Change::Command => value[0]["Config"]["Cmd"] = serde_json::json!([OTHER_COMMAND]),
            Change::ForeignTmpfs | Change::ValidTmpfs => {
                let destination = if settles {
                    CONTAINER_TEMP_MOUNT
                } else {
                    OTHER_TEMP
                };
                value[0]["Mounts"].as_array_mut().unwrap().push(serde_json::json!({
                    "Type": CONTAINER_TMPFS_KIND, "Source": "", "Destination": destination, "RW": true,
                }));
            }
        }
        docker.write("inspect", &serde_json::to_vec(&value).unwrap());
        docker.write("list", CONTAINER_ID.as_bytes());
        docker.write("settle-after-remove", b"");
        let error = owner
            .finish(Err(anyhow::anyhow!(CLIENT_FAILURE)))
            .unwrap_err();
        let removal = format!("container rm --force {CONTAINER_ID}");
        assert_eq!(docker.calls().contains(&removal), settles);
        if settles {
            assert_eq!(error.to_string(), CLIENT_FAILURE);
            assert!(docker.calls().contains(&format!(
                "container ls --all --no-trunc --quiet --filter id={CONTAINER_ID}"
            )));
        } else {
            assert!(format!("{error:#}").contains(CONTAINER_CHANGED));
        }
        drop(snapshot);
        assert_eq!(root.exists(), !settles);
        if !settles {
            assert_eq!(fs::read(&evidence).unwrap(), EVIDENCE_BYTES);
            assert_eq!(
                fs::metadata(evidence).unwrap().permissions().mode() & MODE_MASK,
                mode
            );
            remove_preserved(root);
        }
    }
}

#[test]
fn inaccessible_cid_parent_is_an_inspection_error_and_never_admits_an_owner() {
    let snapshot = GateSnapshot::fixture().unwrap();
    let engine = TestEngine::default();
    let expected = contract(&snapshot);
    let temporary = snapshot.temporary();
    let original_mode = temporary.mode().unwrap() & MODE_MASK;
    temporary.set_mode(NO_ACCESS).unwrap();
    let result = Owner::prepare(&snapshot, &engine, expected, None);
    temporary.set_mode(original_mode).unwrap();
    assert!(result.is_err());
    assert!(engine.lookups.borrow().is_empty());
    assert!(engine.removals.borrow().is_empty());
    drop(result);
    snapshot.close().unwrap();
}
