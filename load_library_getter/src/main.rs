use windows_sys::{
    Win32::System::LibraryLoader::{GetModuleHandleW, GetProcAddress},
    s, w,
};

fn main() {
    const KERNEL_32_DLL: windows_sys::core::PCWSTR = w!("kernel32.dll");
    const LOAD_LIBRARY_A_C: windows_sys::core::PCSTR = s!("LoadLibraryW");

    let module = unsafe { GetModuleHandleW(KERNEL_32_DLL) };

    let load_library_ptr = unsafe { GetProcAddress(module, LOAD_LIBRARY_A_C) }
        .expect("kernel32.dll always contains LoadLibraryW");

    print!("{}", (load_library_ptr as usize))
}
