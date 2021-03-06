// Copyright 2014 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use prelude::v1::*;

use boxed;
use cmp;
use io;
use ptr;
use libc;
use libc::types::os::arch::extra::{LPSECURITY_ATTRIBUTES, SIZE_T, BOOL,
                                   LPVOID, DWORD, LPDWORD, HANDLE};
use thunk::Thunk;
use sys_common::stack::RED_ZONE;
use sys_common::thread::*;

pub type rust_thread = HANDLE;
pub type rust_thread_return = DWORD;

pub type StartFn = extern "system" fn(*mut libc::c_void) -> rust_thread_return;

#[no_stack_check]
pub extern "system" fn thread_start(main: *mut libc::c_void) -> rust_thread_return {
    return start_thread(main);
}

pub mod guard {
    pub unsafe fn main() -> uint {
        0
    }

    pub unsafe fn current() -> uint {
        0
    }

    pub unsafe fn init() {
    }
}

pub unsafe fn create(stack: uint, p: Thunk) -> io::Result<rust_thread> {
    let raw_p = boxed::into_raw(box p);
    let arg = raw_p as *mut libc::c_void;
    // FIXME On UNIX, we guard against stack sizes that are too small but
    // that's because pthreads enforces that stacks are at least
    // PTHREAD_STACK_MIN bytes big.  Windows has no such lower limit, it's
    // just that below a certain threshold you can't do anything useful.
    // That threshold is application and architecture-specific, however.
    // For now, the only requirement is that it's big enough to hold the
    // red zone.  Round up to the next 64 kB because that's what the NT
    // kernel does, might as well make it explicit.  With the current
    // 20 kB red zone, that makes for a 64 kB minimum stack.
    let stack_size = (cmp::max(stack, RED_ZONE) + 0xfffe) & (-0xfffe - 1);
    let ret = CreateThread(ptr::null_mut(), stack_size as libc::size_t,
                           thread_start, arg, 0, ptr::null_mut());

    if ret as uint == 0 {
        // be sure to not leak the closure
        let _p: Box<Thunk> = Box::from_raw(raw_p);
        Err(io::Error::last_os_error())
    } else {
        Ok(ret)
    }
}

pub unsafe fn set_name(_name: &str) {
    // Windows threads are nameless
    // The names in MSVC debugger are obtained using a "magic" exception,
    // which requires a use of MS C++ extensions.
    // See https://msdn.microsoft.com/en-us/library/xcb2z8hs.aspx
}

pub unsafe fn join(native: rust_thread) {
    use libc::consts::os::extra::INFINITE;
    WaitForSingleObject(native, INFINITE);
}

pub unsafe fn detach(native: rust_thread) {
    assert!(libc::CloseHandle(native) != 0);
}

pub unsafe fn yield_now() {
    // This function will return 0 if there are no other threads to execute,
    // but this also means that the yield was useless so this isn't really a
    // case that needs to be worried about.
    SwitchToThread();
}

#[allow(non_snake_case)]
extern "system" {
    fn CreateThread(lpThreadAttributes: LPSECURITY_ATTRIBUTES,
                    dwStackSize: SIZE_T,
                    lpStartAddress: StartFn,
                    lpParameter: LPVOID,
                    dwCreationFlags: DWORD,
                    lpThreadId: LPDWORD) -> HANDLE;
    fn WaitForSingleObject(hHandle: HANDLE, dwMilliseconds: DWORD) -> DWORD;
    fn SwitchToThread() -> BOOL;
}
