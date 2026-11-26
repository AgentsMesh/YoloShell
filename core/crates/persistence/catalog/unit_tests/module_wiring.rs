#[path = "artifact_tests.rs"]
mod artifact_tests;
#[path = "artifacts_quarantine_tests.rs"]
mod artifacts_quarantine_tests;
#[path = "../test_support/catalog_alias_maintenance_hooks.rs"]
mod catalog_alias_maintenance_hooks;
#[path = "catalog_alias_maintenance_hooks_tests.rs"]
mod catalog_alias_maintenance_hooks_tests;
#[path = "catalog_alias_maintenance_tests.rs"]
mod catalog_alias_maintenance_tests;
#[path = "catalog_alias_scanner_adversarial_tests.rs"]
mod catalog_alias_scanner_adversarial_tests;
#[path = "generation_claim_tests.rs"]
mod generation_claim_tests;
#[path = "generation_fence_tests.rs"]
mod generation_fence_tests;
#[path = "generation_tests.rs"]
mod generation_tests;
#[path = "parent_security_tests.rs"]
mod parent_security_tests;
#[path = "retained_namespace_tests.rs"]
mod retained_namespace_tests;
#[path = "retained_registry_tests.rs"]
mod retained_registry_tests;
#[path = "../test_support/retained_test_fixture.rs"]
mod retained_test_fixture;
#[path = "retained_vfs_delete_tests.rs"]
mod retained_vfs_delete_tests;
#[path = "retained_vfs_shm_tests.rs"]
mod retained_vfs_shm_tests;
#[cfg(all(test, target_os = "macos"))]
#[path = "secure_vfs_acl_tests.rs"]
mod secure_vfs_acl_tests;
#[path = "secure_vfs_alias_identity_tests.rs"]
mod secure_vfs_alias_identity_tests;
#[path = "secure_vfs_alias_shard_tests.rs"]
mod secure_vfs_alias_shard_tests;
#[path = "secure_vfs_alias_tests.rs"]
mod secure_vfs_alias_tests;
#[path = "secure_vfs_cleanup_error_tests.rs"]
mod secure_vfs_cleanup_error_tests;
#[path = "secure_vfs_cleanup_tests.rs"]
mod secure_vfs_cleanup_tests;
#[path = "secure_vfs_lock_fence_tests.rs"]
mod secure_vfs_lock_fence_tests;
#[path = "../test_support/secure_vfs_lock_hooks.rs"]
mod secure_vfs_lock_hooks;
#[path = "secure_vfs_open_tests.rs"]
mod secure_vfs_open_tests;
#[path = "secure_vfs_tests.rs"]
mod secure_vfs_tests;
#[path = "tests.rs"]
mod tests;
