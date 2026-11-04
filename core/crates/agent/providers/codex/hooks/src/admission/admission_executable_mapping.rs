#[cfg(target_os = "macos")]
#[path = "admission_executable_mapped_macos.rs"]
mod mapped_macos;
#[cfg(target_os = "linux")]
#[path = "admission_executable_mapped_linux.rs"]
mod mapped_linux;
#[cfg(not(any(target_os = "macos", target_os = "linux")))]
#[path = "admission_unmapped.rs"]
mod unmapped;
