use super::server_binding_repair_test_support::{
    BindingDamage, assert_revoked_authority_stays_bindingless,
    assert_worker_preserves_binding_damage,
};

#[tokio::test]
async fn worker_startup_preserves_a_stale_binding_for_host_repair() {
    assert_worker_preserves_binding_damage(BindingDamage::Running).await;
}

#[tokio::test]
async fn worker_startup_preserves_a_missing_binding_for_host_repair() {
    assert_worker_preserves_binding_damage(BindingDamage::Missing).await;
}

#[tokio::test]
async fn worker_recovery_cannot_resurrect_a_revoked_authority() {
    assert_revoked_authority_stays_bindingless().await;
}
