use std::borrow::BorrowMut;
use std::process::{Child, Command};

pub(crate) fn spawn_waitable_test_child(mut command: impl BorrowMut<Command>) -> Child {
    let _spawn_guard =
        yolo_process_spawn::waitable_child_spawn_guard().expect("acquire test child spawn guard");
    command
        .borrow_mut()
        .spawn()
        .expect("spawn waitable test child")
}
