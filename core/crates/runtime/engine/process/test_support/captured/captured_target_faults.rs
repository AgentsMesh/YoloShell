const SIGNAL_FAILURE_ALREADY_INSTALLED_DETAIL: &str =
    "captured-target signal failure is already installed";
const SIGNAL_NOOP_ALREADY_INSTALLED_DETAIL: &str =
    "captured-target signal no-op is already installed";

struct TestSignalFailure {
    signal: i32,
    target_index: usize,
    detail: String,
}

struct TestSignalNoop {
    signal: i32,
    target_index: usize,
}

thread_local! {
    static TEST_SIGNAL_FAILURE: std::cell::RefCell<Option<TestSignalFailure>> = const {
        std::cell::RefCell::new(None)
    };
    static TEST_SIGNAL_NOOP: std::cell::RefCell<Option<TestSignalNoop>> = const {
        std::cell::RefCell::new(None)
    };
}

pub struct TestSignalFailureGuard(std::marker::PhantomData<std::rc::Rc<()>>);
pub struct TestSignalNoopGuard(std::marker::PhantomData<std::rc::Rc<()>>);

pub fn install_test_signal_failure(
    signal: i32,
    target_index: usize,
    detail: &str,
) -> TestSignalFailureGuard {
    TEST_SIGNAL_FAILURE.with(|slot| {
        let previous = slot.borrow_mut().replace(TestSignalFailure {
            signal,
            target_index,
            detail: detail.to_owned(),
        });
        assert!(
            previous.is_none(),
            "{SIGNAL_FAILURE_ALREADY_INSTALLED_DETAIL}"
        );
    });
    TestSignalFailureGuard(std::marker::PhantomData)
}

pub fn install_test_signal_noop(signal: i32, target_index: usize) -> TestSignalNoopGuard {
    TEST_SIGNAL_NOOP.with(|slot| {
        let previous = slot.borrow_mut().replace(TestSignalNoop {
            signal,
            target_index,
        });
        assert!(previous.is_none(), "{SIGNAL_NOOP_ALREADY_INSTALLED_DETAIL}");
    });
    TestSignalNoopGuard(std::marker::PhantomData)
}

impl Drop for TestSignalFailureGuard {
    fn drop(&mut self) {
        TEST_SIGNAL_FAILURE.with(|slot| *slot.borrow_mut() = None);
    }
}

impl Drop for TestSignalNoopGuard {
    fn drop(&mut self) {
        TEST_SIGNAL_NOOP.with(|slot| *slot.borrow_mut() = None);
    }
}
