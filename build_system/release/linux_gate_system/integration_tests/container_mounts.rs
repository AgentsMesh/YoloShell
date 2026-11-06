use crate::container::model::{Configuration, Contract, Inspection, Mount};
use std::collections::BTreeMap;
use std::path::Path;
use yolo_release_contracts::*;
use yolo_release_gate_system::{
    GateSnapshot, LINUX_AMD64_PROFILE, linux_cgroup_gate_command, linux_gate_command,
};

const BAZEL_PATH: &str = "/fixture/linux-bazel";
const USER: &str = "501:20";
const ID: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
const FOREIGN_SOURCE: &str = "/foreign/source";
const FOREIGN_DESTINATION: &str = "/foreign/destination";
const FOREIGN_KIND: &str = "volume";
const BUILD_ARTIFACT_DIRECTORY: &str = "/yolo-artifacts";

fn bind(source: impl Into<String>, destination: &str, writable: bool) -> Mount {
    Mount {
        kind: CONTAINER_BIND_KIND.into(),
        source: source.into(),
        destination: destination.into(),
        writable,
    }
}

#[test]
fn engine_inspection_accepts_only_the_fixed_mount_set_for_each_role() {
    let snapshot = GateSnapshot::fixture().unwrap();
    let execution = snapshot.execution().unwrap();
    let profile = LINUX_AMD64_PROFILE;
    let bazel = Path::new(BAZEL_PATH);
    for role in [CONTAINER_BUILD_ROLE, CONTAINER_CGROUP_ROLE] {
        let arguments = if role == CONTAINER_BUILD_ROLE {
            linux_gate_command::arguments(profile, &execution, bazel, USER)
        } else {
            linux_cgroup_gate_command::arguments(profile, &execution, bazel, USER)
        }
        .unwrap();
        let image_position = arguments
            .iter()
            .position(|value| value == profile.image)
            .unwrap();
        let contract = Contract::new(profile, &execution, bazel, role, &arguments).unwrap();
        let mut mounts = vec![
            bind(&execution.workspace_snapshot, "/workspace", false),
            bind(BAZEL_PATH, "/yolo-tools/bazel", false),
        ];
        if role == CONTAINER_BUILD_ROLE {
            mounts.push(bind(
                format!("{}/{}", execution.isolated_tmp, profile.docker_architecture),
                BUILD_ARTIFACT_DIRECTORY,
                true,
            ));
        }
        let valid = Inspection {
            id: ID.into(),
            name: format!("/{}", contract.command.name),
            config: Configuration {
                image: profile.image.into(),
                labels: BTreeMap::from([(
                    CONTAINER_OWNER_LABEL.into(),
                    contract.command.owner.clone(),
                )]),
                cmd: arguments[image_position + 1..].to_vec(),
            },
            mounts,
        };
        assert!(contract.verify(&valid).is_ok());
        let mut reordered = valid.clone();
        reordered.mounts.reverse();
        assert!(contract.verify(&reordered).is_ok());
        let mutations: &[fn(&mut Vec<Mount>)] = &[
            |mounts| mounts.first_mut().unwrap().source = FOREIGN_SOURCE.into(),
            |mounts| mounts.first_mut().unwrap().destination = FOREIGN_DESTINATION.into(),
            |mounts| mounts.first_mut().unwrap().writable = true,
            |mounts| mounts.first_mut().unwrap().kind = FOREIGN_KIND.into(),
            |mounts| mounts.push(mounts.first().unwrap().clone()),
            |mounts| {
                mounts.pop();
            },
            |mounts| {
                *mounts.last_mut().unwrap() = mounts.first().unwrap().clone();
            },
        ];
        for mutate in mutations {
            let mut observed = valid.clone();
            mutate(&mut observed.mounts);
            assert!(contract.verify(&observed).is_err());
        }
    }
}
