use std::cell::RefCell;
use std::collections::VecDeque;
use std::marker::PhantomData;
use std::rc::Rc;

const HOOK_ALREADY_INSTALLED_DETAIL: &str = "captured-target signal hook is already installed";
const HOOK_NOT_INSTALLED_DETAIL: &str = "captured-target signal hook is not installed";
const HOOK_OPERATION_MISSING_DETAIL: &str = "captured-target signal hook has no operation";

type SignalHookOperation = Box<dyn FnOnce()>;

struct SignalHook {
    signal: i32,
    target_index: usize,
    operations: VecDeque<SignalHookOperation>,
}

thread_local! {
    static SIGNAL_HOOK: RefCell<Option<SignalHook>> = const { RefCell::new(None) };
}

pub struct TestSignalHookGuard(PhantomData<Rc<()>>);

pub fn install_test_signal_hook(
    signal: i32,
    target_index: usize,
    operation: impl FnOnce() + 'static,
) -> TestSignalHookGuard {
    SIGNAL_HOOK.with(|slot| {
        let mut operations = VecDeque::<SignalHookOperation>::new();
        operations.push_back(Box::new(operation));
        let previous = slot.borrow_mut().replace(SignalHook {
            signal,
            target_index,
            operations,
        });
        assert!(previous.is_none(), "{HOOK_ALREADY_INSTALLED_DETAIL}");
    });
    TestSignalHookGuard(PhantomData)
}

impl TestSignalHookGuard {
    pub fn then(self, operation: impl FnOnce() + 'static) -> Self {
        SIGNAL_HOOK.with(|slot| {
            slot.borrow_mut()
                .as_mut()
                .expect(HOOK_NOT_INSTALLED_DETAIL)
                .operations
                .push_back(Box::new(operation));
        });
        self
    }
}

pub(super) fn run(signal: i32, target_index: usize) {
    let operation = SIGNAL_HOOK.with(|slot| {
        let mut slot = slot.borrow_mut();
        let matches = slot
            .as_ref()
            .is_some_and(|hook| hook.signal == signal && hook.target_index == target_index);
        if !matches {
            return None;
        }
        let operation = slot
            .as_mut()
            .expect(HOOK_NOT_INSTALLED_DETAIL)
            .operations
            .pop_front()
            .expect(HOOK_OPERATION_MISSING_DETAIL);
        if slot.as_ref().is_some_and(|hook| hook.operations.is_empty()) {
            slot.take();
        }
        Some(operation)
    });
    if let Some(operation) = operation {
        operation();
    }
}

impl Drop for TestSignalHookGuard {
    fn drop(&mut self) {
        SIGNAL_HOOK.with(|slot| *slot.borrow_mut() = None);
    }
}
