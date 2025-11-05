#![no_std]
#![no_main]
#![no_implicit_prelude]

extern crate core;
extern crate windows_sys;

use core::{panic::PanicInfo, ptr::null_mut};

use windows_sys::Win32::{
    Storage::FileSystem::WriteFile,
    System::{
        Console::{GetStdHandle, STD_OUTPUT_HANDLE},
        LibraryLoader::{GetModuleHandleA, GetProcAddress},
        Threading::ExitProcess,
    },
};

#[unsafe(no_mangle)]
unsafe extern "C" fn main() -> i32 {
    // Constants for `Kernel32.dll` and `LoadLibraryW`, the dll and functions needed to inject
    const KERNEL_32_DLL: windows_sys::core::PCSTR = c"kernel32.dll".as_ptr().cast();
    const LOAD_LIBRARY_A_C: windows_sys::core::PCSTR = c"LoadLibraryW".as_ptr().cast();

    let module = unsafe { GetModuleHandleA(KERNEL_32_DLL) };
    // SAFETY: The `kernel32.dll` module CANNOT be null, and it will always have LoadLibraryW
    let load_library_ptr = unsafe { GetProcAddress(module, LOAD_LIBRARY_A_C).unwrap_unchecked() };

    // Convert the ptr address to a number, so it can be parsed by the injector
    let mut buf = [0; 20];
    let str = usize_to_str((load_library_ptr as *const ()).addr(), &mut buf);

    // Write the number to stdout so it can be read by the injector
    unsafe {
        WriteFile(
            GetStdHandle(STD_OUTPUT_HANDLE),
            str.as_ptr(),
            str.len() as u32,
            null_mut(),
            null_mut(),
        )
    };

    // Return 0 so it doesn't error
    0
}

// Simple function to convert a `usize` to an `&str`
fn usize_to_str(mut num: usize, buf: &mut [u8; 20]) -> &[u8] {
    let mut i = 20;
    while num > 0 {
        i -= 1;
        buf[i] = b'0' + (num % 10) as u8;
        num /= 10;
    }

    &buf[i..]
}

// Panic handler, always bail.
#[panic_handler]
unsafe fn panic(_info: &PanicInfo) -> ! {
    unsafe { ExitProcess(0xdeafbeef) };
}

#[cfg(target_env = "gnu")]
// This is a lang item which *can* be defined on nightly, or using an extern "C" function.
// It is used for panic unwinding, but all the code here will abort on error (Though errors don't happen)
// So while this code is never used, we're still linking to core, and as such, need it to exist on GNU targets.
#[unsafe(no_mangle)]
extern "C" fn rust_eh_personality() {
    // There is a single point where the program could panic, but it won't.
    unsafe { ExitProcess(0x100) };
}

// used as entry points for windows
#[cfg(target_env = "msvc")]
#[link(name = "msvcrt")]
unsafe extern "C" {}

#[cfg(target_env = "msvc")]
#[link(name = "vcruntime")]
unsafe extern "C" {}

#[cfg(target_env = "msvc")]
#[link(name = "ucrt")]
unsafe extern "C" {}
