mod constants;
use super::*;
use crate::provider_fixture as fixture;
use constants::*;

fn process(executable: &str, argv: &[&str]) -> ObservedProcess {
    ObservedProcess {
        pid: 1,
        start_time: 1,
        parent_pid: 0,
        process_group: 1,
        foreground_group: 1,
        executable: executable.into(),
        argv: argv.iter().map(|value| (*value).into()).collect(),
        environment: None,
        cwd: None,
        evidence_paths: None,
    }
}

#[test]
fn exact_executables_and_node_entrypoints_are_strict() {
    assert!(matches_executable_or_node_entrypoint(
        &process(fixture::ABSOLUTE_EXECUTABLE, fixture::EXECUTABLE_NAMES),
        fixture::EXECUTABLE_NAMES,
        fixture::NODE_PACKAGE_COMPONENTS,
        fixture::NODE_ENTRYPOINTS,
    ));
    assert!(matches_executable_or_node_entrypoint(
        &process(
            NODE_EXECUTABLES[0],
            &[NODE_EXECUTABLES[0], fixture::NODE_ENTRYPOINT],
        ),
        fixture::EXECUTABLE_NAMES,
        fixture::NODE_PACKAGE_COMPONENTS,
        fixture::NODE_ENTRYPOINTS,
    ));
    assert!(matches_executable_or_node_entrypoint(
        &process(
            NODE_EXECUTABLES[0],
            &[
                NODE_EXECUTABLES[0],
                NODE_MEMORY_OPTION,
                fixture::NODE_ENTRYPOINT,
                NODE_FLAG
            ],
        ),
        fixture::EXECUTABLE_NAMES,
        fixture::NODE_PACKAGE_COMPONENTS,
        fixture::NODE_ENTRYPOINTS,
    ));
    assert!(!matches_executable_or_node_entrypoint(
        &process(
            NODE_EXECUTABLES[0],
            &[
                NODE_EXECUTABLES[0],
                NODE_MEMORY_OPTION,
                NODE_NO_WARNINGS,
                NODE_TRACE_WARNINGS,
                NODE_SOURCE_MAPS,
                NODE_FLAG,
                NODE_FLAG,
                NODE_FLAG,
                NODE_FLAG,
                fixture::NODE_ENTRYPOINT,
            ],
        ),
        fixture::EXECUTABLE_NAMES,
        fixture::NODE_PACKAGE_COMPONENTS,
        fixture::NODE_ENTRYPOINTS,
    ));
    for candidate in [
        process("python", &["python", fixture::NODE_ENTRYPOINT]),
        process(NODE_EXECUTABLES[0], &[NODE_EXECUTABLES[0]]),
        process(
            NODE_EXECUTABLES[0],
            &[NODE_EXECUTABLES[0], fixture::OTHER_NODE_ENTRYPOINT],
        ),
        process(
            NODE_EXECUTABLES[0],
            &[NODE_EXECUTABLES[0], fixture::HELPER_NODE_ENTRYPOINT],
        ),
    ] {
        assert!(!matches_executable_or_node_entrypoint(
            &candidate,
            fixture::EXECUTABLE_NAMES,
            fixture::NODE_PACKAGE_COMPONENTS,
            fixture::NODE_ENTRYPOINTS,
        ));
    }
}
