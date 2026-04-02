//! Win32 subsystem.

pub mod cmd;
pub mod compositor;
pub mod conhost;
pub mod console;
pub mod csrss_host;
pub mod csrss_proto;
pub mod msg_dispatch;
pub mod register;
pub mod shell_bringup;
pub mod dwrite;
pub mod exec;
pub mod gdi32;
pub mod subsystem;
#[cfg(target_arch = "x86_64")]
pub mod syscall_win32;
#[cfg(not(target_arch = "x86_64"))]
pub mod syscall_win32 {
    /// No LSTAR syscall table on this architecture yet.
    pub fn register_win32_syscalls_bringup() {}
}
pub mod text_bringup;
pub mod user32;
pub mod win32_paint;
pub mod window_surface;
pub mod windowing;
pub mod wow64;
pub mod input_win32;
pub mod compositor_ipc;
