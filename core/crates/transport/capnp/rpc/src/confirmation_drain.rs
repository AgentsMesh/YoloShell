use std::{cell::Cell, rc::Rc};
use tokio::sync::Notify;

#[derive(Default)]
pub(crate) struct Drain {
    active: Cell<usize>,
    changed: Notify,
}
impl Drain {
    pub fn begin(self: &Rc<Self>) -> Guard {
        self.active.set(self.active.get() + 1);
        Guard(self.clone())
    }
    pub async fn finish(&self) {
        loop {
            let changed = self.changed.notified();
            if self.active.get() == 0 {
                return;
            }
            changed.await;
        }
    }
}
pub(crate) struct Guard(Rc<Drain>);
impl Drop for Guard {
    fn drop(&mut self) {
        self.0.active.set(self.0.active.get() - 1);
        self.0.changed.notify_one();
    }
}
