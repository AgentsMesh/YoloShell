use super::*;

#[path = "private_file_branch_matrix.rs"]
mod branch_matrix;
#[path = "private_file_cleanup_fault_matrix.rs"]
mod cleanup_fault_matrix;
#[path = "private_file_instance_matrix.rs"]
mod instance_matrix;
#[path = "private_file_parent_lock_tests.rs"]
mod parent_lock_tests;
#[path = "private_file_publish_safety_tests.rs"]
mod publish_safety_tests;
#[path = "private_file_retained_lock_tests.rs"]
mod retained_lock_tests;
#[path = "private_file_retained_recovery_tests.rs"]
mod retained_recovery_tests;
#[path = "private_file_slot_race_matrix.rs"]
mod slot_race_matrix;
#[path = "private_file_target_lock_tests.rs"]
mod target_lock_tests;
#[path = "private_file_temp_prepare_matrix.rs"]
mod temp_prepare_matrix;
#[path = "private_file_temporary_race_matrix.rs"]
mod temporary_race_matrix;
#[path = "private_file_unlink_proof_tests.rs"]
mod unlink_proof_tests;
