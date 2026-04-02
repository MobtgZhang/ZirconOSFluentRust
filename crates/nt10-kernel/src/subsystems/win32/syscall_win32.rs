//! Win32 bring-up syscalls — registered with [`crate::arch::x86_64::syscall::zr_syscall_register`].

use crate::arch::x86_64::syscall::zr_syscall_register;
use crate::libs::win32_abi::{Hwnd, LParam, WParam};

use super::msg_dispatch::{
    self, ZR_SYSCALL_CREATE_WINDOW_EX, ZR_SYSCALL_DISPATCH_MESSAGE, ZR_SYSCALL_GET_MESSAGE,
    ZR_SYSCALL_POST_MESSAGE, ZR_SYSCALL_SEND_MESSAGE,
};
use super::windowing::{create_window_ex_on_desktop, def_window_proc_bringup};

fn sc_create_window_ex(a1: u64, a2: u64, _a3: u64, _a4: u64, _a5: u64, _a6: u64) -> i32 {
    let tid = msg_dispatch::current_thread_for_win32();
    let Some(dptr) = msg_dispatch::thread_desktop_ptr(tid) else {
        return -1;
    };
    let desktop = unsafe { dptr.as_ref() };
    match create_window_ex_on_desktop(
        desktop,
        a1 as u16,
        a2 as Hwnd,
        tid,
        def_window_proc_bringup,
    ) {
        Ok(h) => h as i32,
        Err(()) => -1,
    }
}

fn sc_post_message(a1: u64, a2: u64, a3: u64, a4: u64, _a5: u64, _a6: u64) -> i32 {
    let tid = msg_dispatch::current_thread_for_win32();
    let Some(dptr) = msg_dispatch::thread_desktop_ptr(tid) else {
        return -1;
    };
    let desktop = unsafe { dptr.as_ref() };
    match msg_dispatch::post_message_kernel(
        desktop,
        a1 as Hwnd,
        a2 as u32,
        a3 as WParam,
        a4 as LParam,
    ) {
        Ok(()) => 0,
        Err(()) => -1,
    }
}

fn sc_get_message(_a1: u64, _a2: u64, _a3: u64, _a4: u64, _a5: u64, _a6: u64) -> i32 {
    let tid = msg_dispatch::current_thread_for_win32();
    let _m = msg_dispatch::get_message_wait_kernel(tid);
    0
}

fn sc_send_message(a1: u64, a2: u64, a3: u64, a4: u64, _a5: u64, _a6: u64) -> i32 {
    let tid = msg_dispatch::current_thread_for_win32();
    let Some(dptr) = msg_dispatch::thread_desktop_ptr(tid) else {
        return -1;
    };
    let desktop = unsafe { dptr.as_ref() };
    match msg_dispatch::send_message_kernel(
        tid,
        desktop,
        a1 as Hwnd,
        a2 as u32,
        a3 as WParam,
        a4 as LParam,
    ) {
        Ok(r) => r as i32,
        Err(()) => -1,
    }
}

fn sc_dispatch_message(_a1: u64, _a2: u64, _a3: u64, _a4: u64, _a5: u64, _a6: u64) -> i32 {
    let tid = msg_dispatch::current_thread_for_win32();
    let Some(dptr) = msg_dispatch::thread_desktop_ptr(tid) else {
        return -1;
    };
    let Some(m) = msg_dispatch::take_last_get_message() else {
        return -1;
    };
    let desktop = unsafe { dptr.as_ref() };
    msg_dispatch::dispatch_message_kernel(desktop, m) as i32
}

/// Install handlers (idempotent best-effort: ignores double-register errors).
pub fn register_win32_syscalls_bringup() {
    let _ = zr_syscall_register(ZR_SYSCALL_CREATE_WINDOW_EX, sc_create_window_ex);
    let _ = zr_syscall_register(ZR_SYSCALL_POST_MESSAGE, sc_post_message);
    let _ = zr_syscall_register(ZR_SYSCALL_GET_MESSAGE, sc_get_message);
    let _ = zr_syscall_register(ZR_SYSCALL_DISPATCH_MESSAGE, sc_dispatch_message);
    let _ = zr_syscall_register(ZR_SYSCALL_SEND_MESSAGE, sc_send_message);
}
