use super::super::limits::{
    MAX_PROCESS_SNAPSHOT_ARGV_ITEMS, MAX_PROCESS_SNAPSHOT_EVIDENCE_ITEMS,
    MAX_PROCESS_SNAPSHOT_TOTAL_ARGV_ITEMS, MAX_PROCESS_SNAPSHOT_TOTAL_EVIDENCE_ITEMS,
};
use super::*;

const TEST_EMPTY_ARGUMENT: &str = "";

#[test]
fn stable_snapshot_rejects_excessive_global_argument_count() {
    let process_count = MAX_PROCESS_SNAPSHOT_TOTAL_ARGV_ITEMS
        .checked_div(MAX_PROCESS_SNAPSHOT_ARGV_ITEMS)
        .unwrap()
        .saturating_add(LIMIT_EXCESS);
    let processes = (0..process_count)
        .map(|index| {
            let mut process = valid_process(TEST_PID + u32::try_from(index).unwrap());
            process.argv = vec![TEST_EMPTY_ARGUMENT.to_owned(); MAX_PROCESS_SNAPSHOT_ARGV_ITEMS];
            process
        })
        .collect();

    assert!(decode_processes(processes, &[]).is_err());
}

#[test]
fn stable_snapshot_rejects_excessive_global_evidence_count() {
    let process_count = MAX_PROCESS_SNAPSHOT_TOTAL_EVIDENCE_ITEMS
        .checked_div(MAX_PROCESS_SNAPSHOT_EVIDENCE_ITEMS)
        .unwrap()
        .saturating_add(LIMIT_EXCESS);
    let processes = (0..process_count)
        .map(|index| {
            let mut process = valid_process(TEST_PID + u32::try_from(index).unwrap());
            process.evidence_paths = Some(
                (0..MAX_PROCESS_SNAPSHOT_EVIDENCE_ITEMS)
                    .map(|path| {
                        PathBuf::from(format!("{TEST_EVIDENCE_PREFIX}{index}-{path}")).into()
                    })
                    .collect(),
            );
            process
        })
        .collect();

    assert!(decode_processes(processes, &[]).is_err());
}
