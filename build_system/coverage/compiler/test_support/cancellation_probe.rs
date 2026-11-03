use std::future::{Future, pending};
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::task::{Context, Poll, Waker};

const PANIC_PAYLOAD: u8 = 42;
const ONCE: usize = 1;
const NEVER: usize = 0;
static STARTED: AtomicUsize = AtomicUsize::new(NEVER);
static RESUMED: AtomicUsize = AtomicUsize::new(NEVER);
static DROP_ENTERED: AtomicUsize = AtomicUsize::new(NEVER);
static DROP_RETURNED: AtomicUsize = AtomicUsize::new(NEVER);

#[unsafe(no_mangle)]
#[inline(never)]
pub fn yolo_probe_after_await() {
    RESUMED.fetch_add(ONCE, Ordering::SeqCst);
}

#[unsafe(no_mangle)]
#[inline(never)]
pub fn yolo_probe_after_drop() {
    DROP_RETURNED.fetch_add(ONCE, Ordering::SeqCst);
}

#[inline(never)]
async fn suspended() {
    STARTED.fetch_add(ONCE, Ordering::SeqCst);
    pending::<()>().await;
    yolo_probe_after_await();
}

struct WitnessDrop;
impl Drop for WitnessDrop {
    fn drop(&mut self) {
        DROP_ENTERED.fetch_add(ONCE, Ordering::SeqCst);
        if std::hint::black_box(true) {
            std::panic::panic_any(PANIC_PAYLOAD);
        }
    }
}

#[inline(never)]
fn unwinding_drop() {
    let guard = WitnessDrop;
    drop(guard);
    yolo_probe_after_drop();
}

fn main() {
    std::panic::set_hook(Box::new(|_| {}));
    let mut future = Box::pin(suspended());
    let mut context = Context::from_waker(Waker::noop());
    assert!(matches!(future.as_mut().poll(&mut context), Poll::Pending));
    drop(future);
    assert_eq!(STARTED.load(Ordering::SeqCst), ONCE);
    assert_eq!(RESUMED.load(Ordering::SeqCst), NEVER);
    let result = catch_unwind(AssertUnwindSafe(unwinding_drop));
    let payload = result.expect_err("destructor must unwind");
    assert_eq!(payload.downcast_ref::<u8>(), Some(&PANIC_PAYLOAD));
    assert_eq!(DROP_ENTERED.load(Ordering::SeqCst), ONCE);
    assert_eq!(DROP_RETURNED.load(Ordering::SeqCst), NEVER);
    println!("await_started=1; await_resumed=0; drop_entered=1; drop_returned=0");
}
