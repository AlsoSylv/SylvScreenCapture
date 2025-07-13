#![no_std]
#![no_main]
#![no_implicit_prelude]

extern crate core;
extern crate windows_sys;

use core::option::Option::Some;
use core::{panic::PanicInfo, ptr::null_mut};

use windows_sys::{
    Win32::{
        Storage::FileSystem::WriteFile,
        System::{
            Console::{GetStdHandle, STD_OUTPUT_HANDLE},
            LibraryLoader::{GetModuleHandleW, GetProcAddress},
            Threading::ExitProcess,
        },
    },
    s, w,
};


#[unsafe(no_mangle)]
unsafe extern "C" fn main() -> i32 {
    // Constants for `Kernel32.dll` and `LoadLibraryW`, the dll and functions needed to inject
    const KERNEL_32_DLL: windows_sys::core::PCWSTR = w!("kernel32.dll");
    const LOAD_LIBRARY_A_C: windows_sys::core::PCSTR = s!("LoadLibraryW");

    let module = unsafe { GetModuleHandleW(KERNEL_32_DLL) };
    let load_library_ptr = unsafe { GetProcAddress(module, LOAD_LIBRARY_A_C) }
        .expect("kernel32.dll always contains LoadLibraryW");

    // Convert the ptr address to a number, so it can be parsed by the injector
    let mut buf = [0; 20];
    let str = usize_to_str((load_library_ptr as *const ()).addr(), &mut buf);

    // Write the number to stdout so it can be read by the injector
    let mut written = 0;
    unsafe {
        WriteFile(
            GetStdHandle(STD_OUTPUT_HANDLE),
            str.as_ptr() as _,
            str.len() as u32,
            &mut written,
            null_mut(),
        )
    };

    // Return 0 so it doesn't error
    return 0;
}

// Simple function to convert a `usize` to an `&str`
fn usize_to_str<'a>(mut num: usize, buf: &'a mut [u8; 20]) -> &'a str {
    if num == 0 {
        buf[19] = b'0';
        unsafe { core::str::from_utf8_unchecked(&buf[19..]) }
    } else {
        let mut i = 20;
        while num > 0 {
            i -= 1;
            buf[i] = b'0' + (num % 10) as u8;
            num /= 10;
        }

        unsafe { core::str::from_utf8_unchecked(&buf[i..]) }
    }
}

// Panic handler, always bail.
#[panic_handler]
unsafe fn panic(_info: &PanicInfo) -> ! {
    unsafe { ExitProcess(0xdeafbeef) };
}

// Other panic handler, always bail.
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
