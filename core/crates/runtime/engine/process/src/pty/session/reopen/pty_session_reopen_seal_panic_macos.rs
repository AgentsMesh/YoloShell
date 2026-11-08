const INJECTED_PANIC_DETAIL: &str = "injected PTY reopen-seal panic";
const SCHEDULED_PANIC_PRESENT_DETAIL: &str = "scheduled reopen-seal panic is present";

#[derive(Clone, Copy, Eq, PartialEq)]
pub(super) enum TestSealOperation {
    Activate,
    Revalidate,
    Commit,
    PrepareMasterAliasClose,
    Restore,
}

thread_local! {
    static NEXT_PANIC: std::cell::Cell<Option<TestSealOperation>> = const {
        std::cell::Cell::new(None)
    };
    static NEXT_DROP_PANIC: std::cell::RefCell<
        Option<(TestSealOperation, std::sync::Arc<std::sync::atomic::AtomicBool>)>
    > = const { std::cell::RefCell::new(None) };
}

pub(super) fn panic_next(operation: TestSealOperation) {
    NEXT_PANIC.with(|next| next.set(Some(operation)));
}

pub(super) fn panic_next_with_payload_drop(
    operation: TestSealOperation,
    dropped: std::sync::Arc<std::sync::atomic::AtomicBool>,
) {
    NEXT_DROP_PANIC.with(|next| assert!(next.borrow_mut().replace((operation, dropped)).is_none()));
}

pub(super) fn panic_if_requested(operation: TestSealOperation) {
    let drop_payload = NEXT_DROP_PANIC.with(|next| {
        let mut next = next.borrow_mut();
        next.as_ref()
            .is_some_and(|(scheduled, _)| *scheduled == operation)
            .then(|| next.take().expect(SCHEDULED_PANIC_PRESENT_DETAIL).1)
    });
    if let Some(dropped) = drop_payload {
        std::panic::panic_any(PanicDropPayload(dropped));
    }
    NEXT_PANIC.with(|next| {
        if next.get() == Some(operation) {
            next.set(None);
            panic!("{INJECTED_PANIC_DETAIL}");
        }
    });
}

struct PanicDropPayload(std::sync::Arc<std::sync::atomic::AtomicBool>);

impl Drop for PanicDropPayload {
    fn drop(&mut self) {
        self.0.store(true, std::sync::atomic::Ordering::SeqCst);
        panic!("{INJECTED_PANIC_DETAIL}");
    }
}
