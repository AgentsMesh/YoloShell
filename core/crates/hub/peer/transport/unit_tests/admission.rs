use crate::{
    ChannelAdmissionError,
    constants::{CHANNEL_LIMIT, CLOSED},
};
use tokio::sync::Semaphore;

const CHANNELS: usize = 1;

#[test]
fn closing_a_full_pool_is_not_transient_capacity_pressure() {
    let pool = Semaphore::new(CHANNELS);
    let held = pool.try_acquire().unwrap();
    let full: anyhow::Error = ChannelAdmissionError::from(pool.try_acquire().unwrap_err()).into();
    assert_eq!(full.to_string(), CHANNEL_LIMIT);
    assert_eq!(
        full.downcast_ref::<ChannelAdmissionError>(),
        Some(&ChannelAdmissionError::AtCapacity)
    );
    pool.close();
    let closed: anyhow::Error = ChannelAdmissionError::from(pool.try_acquire().unwrap_err()).into();
    assert_eq!(closed.to_string(), CLOSED);
    assert_eq!(
        closed.downcast_ref::<ChannelAdmissionError>(),
        Some(&ChannelAdmissionError::Closed)
    );
    drop(held);
    assert_eq!(
        ChannelAdmissionError::from(pool.try_acquire().unwrap_err()),
        ChannelAdmissionError::Closed,
    );
}
