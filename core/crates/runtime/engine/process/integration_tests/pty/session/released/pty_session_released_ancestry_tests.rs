use super::*;
use anyhow::anyhow;

const ROOT_START_TIME: u64 = 80_000;
const PRE_ROOT_START_TIME: u64 = ROOT_START_TIME - START_TIME_STEP;
const POST_ROOT_START_TIME: u64 = ROOT_START_TIME + START_TIME_STEP;
const START_TIME_STEP: u64 = 1;
const FIRST_EXTERNAL_PID: u32 = 81_001;
const SECOND_EXTERNAL_PID: u32 = 81_002;
const FIRST_EXTERNAL_LINEAGE: u64 = 82_001;
const SECOND_EXTERNAL_LINEAGE: u64 = 82_002;
const ROOT_LINEAGE: u64 = 82_003;
const QUERY_FAILURE_DETAIL: &str = "injected external ancestry query failure";

#[test]
fn matching_pre_root_external_parent_is_excluded() {
    let inherited = resolve_fixture(
        FIRST_EXTERNAL_PID,
        FIRST_EXTERNAL_LINEAGE,
        HashMap::from([(
            FIRST_EXTERNAL_PID,
            process(FIRST_EXTERNAL_LINEAGE, PRE_ROOT_START_TIME),
        )]),
    );

    assert!(!inherited);
}

#[test]
fn post_root_external_chain_reaches_pre_root_parent() {
    let mut child = process(FIRST_EXTERNAL_LINEAGE, POST_ROOT_START_TIME);
    child.parent_pid = SECOND_EXTERNAL_PID;
    child.parent_lineage = SECOND_EXTERNAL_LINEAGE;
    let inherited = resolve_fixture(
        FIRST_EXTERNAL_PID,
        FIRST_EXTERNAL_LINEAGE,
        HashMap::from([
            (FIRST_EXTERNAL_PID, child),
            (
                SECOND_EXTERNAL_PID,
                process(SECOND_EXTERNAL_LINEAGE, PRE_ROOT_START_TIME),
            ),
        ]),
    );

    assert!(!inherited);
}

#[test]
fn reparented_generation_mismatch_remains_eligible() {
    let inherited = resolve_fixture(
        FIRST_EXTERNAL_PID,
        FIRST_EXTERNAL_LINEAGE,
        HashMap::from([(
            FIRST_EXTERNAL_PID,
            process(SECOND_EXTERNAL_LINEAGE, PRE_ROOT_START_TIME),
        )]),
    );

    assert!(inherited);
}

#[test]
fn unavailable_external_parent_remains_eligible() {
    let mut cache = HashMap::new();
    let missing = resolve(
        FIRST_EXTERNAL_PID,
        FIRST_EXTERNAL_LINEAGE,
        ROOT_START_TIME,
        ROOT_LINEAGE,
        &mut cache,
        |_pid| Ok(None),
    );
    let failed = resolve(
        SECOND_EXTERNAL_PID,
        SECOND_EXTERNAL_LINEAGE,
        ROOT_START_TIME,
        ROOT_LINEAGE,
        &mut cache,
        |_pid| Err(anyhow!(QUERY_FAILURE_DETAIL)),
    );

    assert!(missing);
    assert!(failed);
}

fn resolve_fixture(pid: u32, lineage: u64, processes: HashMap<u32, ExternalProcess>) -> bool {
    resolve(
        pid,
        lineage,
        ROOT_START_TIME,
        ROOT_LINEAGE,
        &mut HashMap::new(),
        |candidate| Ok(processes.get(&candidate).copied()),
    )
}

fn process(lineage: u64, start_time: u64) -> ExternalProcess {
    ExternalProcess {
        lineage,
        parent_lineage: u64::default(),
        parent_pid: u32::default(),
        start_time,
    }
}
