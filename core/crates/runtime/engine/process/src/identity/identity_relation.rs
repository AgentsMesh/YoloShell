pub use yolo_process_identity::ProcessIdentityRelation;

pub fn process_identity_relation(pid: u32, expected_start_time: u64) -> ProcessIdentityRelation {
    #[cfg(test)]
    if let Some(relation) = super::identity_relation_test_hook::relation(pid, expected_start_time) {
        return relation;
    }
    #[cfg(all(feature = "adversarial-test-hooks", not(test)))]
    if let Some(relation) =
        super::adversarial_identity_relation_test_hook::relation(pid, expected_start_time)
    {
        return relation;
    }
    yolo_process_identity::process_identity_relation(pid, expected_start_time)
}
