use std::io;
use std::process::{Child, Output};

pub(super) struct ChildCustody {
    child: Option<Child>,
}

impl ChildCustody {
    pub(super) fn new(child: Child) -> Self {
        Self { child: Some(child) }
    }

    pub(super) fn child_mut(&mut self) -> &mut Child {
        self.child.as_mut().expect("child custody is armed")
    }

    pub(super) fn finish(mut self) -> io::Result<Output> {
        self.child
            .take()
            .expect("child custody is armed")
            .wait_with_output()
    }
}

impl Drop for ChildCustody {
    fn drop(&mut self) {
        let Some(mut child) = self.child.take() else {
            return;
        };
        match child.try_wait() {
            Ok(Some(_)) => return,
            Ok(None) | Err(_) => {}
        }
        let _ = yolo_runtime::process::terminate_detached_child(child);
    }
}
