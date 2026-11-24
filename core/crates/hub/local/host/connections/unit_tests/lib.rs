use crate::{
    HostConnectionState, HostConnections, INPUT_CONNECTION_IDLE_BUDGET,
};
use std::time::{Duration, Instant};
use uuid::Uuid;

const SHARED_CAPACITY: usize = 2;
const SINGLE_CAPACITY: usize = 1;
const IDLE_MARGIN: Duration = Duration::from_millis(1);

#[tokio::test]
async fn slots_are_reused_for_the_same_runtime() {
    let pool = HostConnections::with_capacity(SHARED_CAPACITY);
    let runtime = Uuid::new_v4();
    let first = pool.slot(runtime).await;
    drop(first);
    let second = pool.slot(runtime).await;
    let again = pool.slot(runtime).await;
    assert_eq!(pool.len().await, 1);
    assert!(std::sync::Arc::ptr_eq(&second, &again));
}

#[tokio::test]
async fn different_runtimes_never_share_a_slot() {
    let pool = HostConnections::with_capacity(SHARED_CAPACITY);
    let first = pool.slot(Uuid::new_v4()).await;
    let second = pool.slot(Uuid::new_v4()).await;
    assert!(!std::sync::Arc::ptr_eq(&first, &second));
}

#[tokio::test]
async fn idle_slots_are_evicted_at_the_bound() {
    let pool = HostConnections::with_capacity(SHARED_CAPACITY);
    let first_id = Uuid::new_v4();
    let second_id = Uuid::new_v4();
    let third_id = Uuid::new_v4();
    drop(pool.slot(first_id).await);
    drop(pool.slot(second_id).await);
    let _ = pool.slot(third_id).await;
    assert_eq!(pool.len().await, SHARED_CAPACITY);
}

#[tokio::test]
async fn an_in_use_slot_is_not_evicted() {
    let pool = HostConnections::with_capacity(SINGLE_CAPACITY);
    let first_id = Uuid::new_v4();
    let first = pool.slot(first_id).await;
    let second_id = Uuid::new_v4();
    let uncached = pool.slot(second_id).await;
    assert!(!std::sync::Arc::ptr_eq(&first, &uncached));
    drop(first);
    let cached = pool.slot(second_id).await;
    assert!(!std::sync::Arc::ptr_eq(&uncached, &cached));
    assert_eq!(pool.len().await, 1);
}

#[test]
fn connection_state_expires_after_the_idle_budget() {
    let now = Instant::now();
    let state = HostConnectionState {
        connection: None,
        last_used: now - INPUT_CONNECTION_IDLE_BUDGET - IDLE_MARGIN,
    };
    assert!(state.idle(now));
    assert!(
        !HostConnectionState {
            connection: None,
            last_used: now,
        }
        .idle(now)
    );
}
