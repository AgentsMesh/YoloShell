use super::*;
use crate::registry::{MAX_RETAINED_CONVERSATIONS, Registry};
use std::sync::Arc;
use yolo_crypto::Sha256Digest;

#[test]
fn separate_conversations_do_not_share_a_read_lock_and_active_context_is_reused() {
    let (binding, target) = binding();
    let address = resolve_conversation(&binding, target, &sources()).unwrap().address;
    let mut other = address.clone();
    other.conversation_id = Sha256Digest::repeated(9);
    let mut registry = Registry::default();
    let first = registry.lease(&address).unwrap();
    let _read_in_progress = first.lock().unwrap();
    let second = registry.lease(&other).unwrap();
    assert!(second.try_lock().is_ok());
    assert!(Arc::ptr_eq(&first, &registry.lease(&address).unwrap()));
}

#[test]
fn saturated_registry_rejects_new_readers_and_never_evicts_a_pinned_generation() {
    let (binding, target) = binding();
    let original = resolve_conversation(&binding, target, &sources()).unwrap().address;
    let mut registry = Registry::default();
    let mut leases = Vec::new();
    for index in 0..MAX_RETAINED_CONVERSATIONS {
        let mut address = original.clone();
        address.conversation_id = Sha256Digest::capture(&index).unwrap();
        leases.push((address.clone(), registry.lease(&address).unwrap()));
    }
    assert!(matches!(
        registry.lease(&original),
        Err(ConversationError::ReaderBusy)
    ));
    let pinned = leases[0].clone();
    assert!(Arc::ptr_eq(&pinned.1, &registry.lease(&pinned.0).unwrap()));
    let (_, idle) = leases.pop().unwrap();
    let retired = Arc::downgrade(&idle);
    drop(idle);
    assert!(registry.lease(&original).is_ok());
    assert!(retired.upgrade().is_none());
    assert!(Arc::ptr_eq(&pinned.1, &registry.lease(&pinned.0).unwrap()));
}
