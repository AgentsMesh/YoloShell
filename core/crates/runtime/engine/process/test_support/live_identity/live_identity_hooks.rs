use std::cell::RefCell;

const MATCH_FAILURE_ALREADY_INSTALLED_DETAIL: &str = "process match failure is already installed";
const SIGNAL_FAILURE_ALREADY_INSTALLED_DETAIL: &str = "process signal failure is already installed";
const SIGNAL_OBSERVER_ALREADY_INSTALLED_DETAIL: &str =
    "process signal observer is already installed";
const STATE_FAILURE_ALREADY_INSTALLED_DETAIL: &str =
    "process execution-state failure is already installed";

thread_local! {
    static TEST_MATCH_FAILURE: RefCell<Option<String>> = const { RefCell::new(None) };
    static TEST_SIGNAL_FAILURE: RefCell<Option<Box<dyn Fn() -> Option<String>>>> =
        const { RefCell::new(None) };
    static TEST_SIGNAL_OBSERVER: RefCell<Option<Box<dyn Fn(i32)>>> = const { RefCell::new(None) };
    static TEST_STATE_FAILURE: RefCell<Option<Box<dyn Fn() -> Option<anyhow::Error>>>> =
        const { RefCell::new(None) };
}

pub struct TestMatchFailureGuard;
pub struct TestSignalFailureGuard;
pub struct TestSignalObserverGuard;
pub struct TestStateFailureGuard;

pub fn install_test_match_failure(detail: &str) -> TestMatchFailureGuard {
    TEST_MATCH_FAILURE.with(|failure| {
        let previous = failure.borrow_mut().replace(detail.to_owned());
        assert!(
            previous.is_none(),
            "{}",
            MATCH_FAILURE_ALREADY_INSTALLED_DETAIL
        );
    });
    TestMatchFailureGuard
}

pub fn install_test_live_signal_failure(
    failure: impl Fn() -> Option<String> + 'static,
) -> TestSignalFailureGuard {
    TEST_SIGNAL_FAILURE.with(|slot| {
        let previous = slot.borrow_mut().replace(Box::new(failure));
        assert!(
            previous.is_none(),
            "{SIGNAL_FAILURE_ALREADY_INSTALLED_DETAIL}"
        );
    });
    TestSignalFailureGuard
}

pub fn install_test_live_signal_observer(
    observer: impl Fn(i32) + 'static,
) -> TestSignalObserverGuard {
    TEST_SIGNAL_OBSERVER.with(|slot| {
        let previous = slot.borrow_mut().replace(Box::new(observer));
        assert!(
            previous.is_none(),
            "{SIGNAL_OBSERVER_ALREADY_INSTALLED_DETAIL}"
        );
    });
    TestSignalObserverGuard
}

pub fn install_test_live_execution_state_failure(
    failure: impl Fn() -> Option<anyhow::Error> + 'static,
) -> TestStateFailureGuard {
    TEST_STATE_FAILURE.with(|slot| {
        let previous = slot.borrow_mut().replace(Box::new(failure));
        assert!(
            previous.is_none(),
            "{STATE_FAILURE_ALREADY_INSTALLED_DETAIL}"
        );
    });
    TestStateFailureGuard
}

pub(super) fn match_failure() -> Option<String> {
    TEST_MATCH_FAILURE.with(|failure| failure.borrow().clone())
}

pub(super) fn signal_failure() -> Option<String> {
    TEST_SIGNAL_FAILURE.with(|failure| failure.borrow().as_ref().and_then(|failure| failure()))
}

pub(super) fn observe_signal(signal: i32) {
    TEST_SIGNAL_OBSERVER.with(|slot| {
        if let Some(observer) = slot.borrow().as_ref() {
            observer(signal);
        }
    });
}

pub(super) fn execution_state_failure() -> Option<anyhow::Error> {
    TEST_STATE_FAILURE.with(|failure| failure.borrow().as_ref().and_then(|failure| failure()))
}

impl Drop for TestMatchFailureGuard {
    fn drop(&mut self) {
        TEST_MATCH_FAILURE.with(|failure| *failure.borrow_mut() = None);
    }
}

impl Drop for TestSignalFailureGuard {
    fn drop(&mut self) {
        TEST_SIGNAL_FAILURE.with(|failure| *failure.borrow_mut() = None);
    }
}

impl Drop for TestSignalObserverGuard {
    fn drop(&mut self) {
        TEST_SIGNAL_OBSERVER.with(|slot| *slot.borrow_mut() = None);
    }
}

impl Drop for TestStateFailureGuard {
    fn drop(&mut self) {
        TEST_STATE_FAILURE.with(|failure| *failure.borrow_mut() = None);
    }
}
