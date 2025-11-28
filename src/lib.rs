#[cfg(target_os = "windows")]
mod windows;
#[cfg(target_os = "windows")]
pub mod zoom;
#[cfg(target_os = "windows")]
pub mod config_manager;

#[ctor::ctor]
fn safe_setup() {
}