#[cfg(target_os = "macos")]
#[path = "macos/macos_exec_observer.rs"]
mod macos_exec_observer;
#[cfg(target_os = "macos")]
#[doc(hidden)]
#[path = "macos/macos_mapped_executable.rs"]
pub mod macos_mapped_executable;
#[cfg(target_os = "macos")]
pub use macos_exec_observer::MacosExecObserver;
#[cfg(target_os = "macos")]
#[path = "process/process_events_macos.rs"]
mod process_events_macos;
#[cfg(target_os = "macos")]
#[path = "scoped/scoped_waitable_child_macos.rs"]
mod scoped_waitable_child_macos;
#[cfg(target_os = "macos")]
#[path = "worker/worker_spawn_macos.rs"]
mod worker_spawn_macos;
