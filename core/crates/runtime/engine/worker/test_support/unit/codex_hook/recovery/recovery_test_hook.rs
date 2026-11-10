use std::cell::RefCell;

thread_local! {
    static BEFORE_RECOVERY: RefCell<Option<Box<dyn FnOnce()>>> = const { RefCell::new(None) };
    static AFTER_ROUTE_INSTALL: RefCell<Option<Box<dyn FnOnce()>>> = const { RefCell::new(None) };
    static BEFORE_ROUND_ACCEPT: RefCell<Option<Box<dyn FnOnce()>>> = const { RefCell::new(None) };
    static BEFORE_SOCKET_PUBLICATION: RefCell<Option<Box<dyn FnOnce()>>> =
        const { RefCell::new(None) };
    static AFTER_SOCKET_PUBLICATION: RefCell<Option<Box<dyn FnOnce()>>> =
        const { RefCell::new(None) };
}

pub(super) fn set_after_socket_publication(hook: impl FnOnce() + 'static) {
    AFTER_SOCKET_PUBLICATION.with(|slot| {
        let previous = slot.replace(Some(Box::new(hook)));
        assert!(previous.is_none());
    });
}

pub(super) fn run_after_socket_publication() {
    AFTER_SOCKET_PUBLICATION.with(|slot| {
        if let Some(hook) = slot.take() {
            hook();
        }
    });
}

pub(super) fn set_before_socket_publication(hook: impl FnOnce() + 'static) {
    BEFORE_SOCKET_PUBLICATION.with(|slot| {
        let previous = slot.replace(Some(Box::new(hook)));
        assert!(previous.is_none());
    });
}

pub(super) fn run_before_socket_publication() {
    BEFORE_SOCKET_PUBLICATION.with(|slot| {
        if let Some(hook) = slot.take() {
            hook();
        }
    });
}

pub(super) fn set_before_round_accept(hook: impl FnOnce() + 'static) {
    BEFORE_ROUND_ACCEPT.with(|slot| {
        let previous = slot.replace(Some(Box::new(hook)));
        assert!(previous.is_none());
    });
}

pub(super) fn run_before_round_accept() {
    BEFORE_ROUND_ACCEPT.with(|slot| {
        if let Some(hook) = slot.take() {
            hook();
        }
    });
}

pub(super) fn set_after_route_install(hook: impl FnOnce() + 'static) {
    AFTER_ROUTE_INSTALL.with(|slot| {
        let previous = slot.replace(Some(Box::new(hook)));
        assert!(previous.is_none());
    });
}

pub(super) fn run_after_route_install() {
    AFTER_ROUTE_INSTALL.with(|slot| {
        if let Some(hook) = slot.take() {
            hook();
        }
    });
}

pub(super) fn set(hook: impl FnOnce() + 'static) {
    BEFORE_RECOVERY.with(|slot| {
        let previous = slot.replace(Some(Box::new(hook)));
        assert!(previous.is_none());
    });
}

pub(super) fn run() {
    BEFORE_RECOVERY.with(|slot| {
        if let Some(hook) = slot.take() {
            hook();
        }
    });
}
