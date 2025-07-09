use std::{
    ffi::{OsStr, OsString},
    path::Path,
    process::Stdio,
    ptr::null_mut,
};

use windows::Win32::{
    Foundation::{DUPLICATE_HANDLE_OPTIONS, DuplicateHandle, HANDLE, HMODULE, MAX_PATH},
    System::{
        Diagnostics::Debug::WriteProcessMemory,
        Memory::{MEM_COMMIT, MEM_RESERVE, PAGE_READWRITE, VirtualAllocEx},
        ProcessStatus::{
            ENUM_PROCESS_MODULES_EX_FLAGS, EnumProcessModulesEx, GetModuleFileNameExW,
        },
        Threading::{
            CreateRemoteThread, IsWow64Process, OpenProcess, PROCESS_ACCESS_RIGHTS,
            PROCESS_CREATE_THREAD, PROCESS_DUP_HANDLE, PROCESS_QUERY_INFORMATION,
            PROCESS_VM_OPERATION, PROCESS_VM_READ, PROCESS_VM_WRITE,
        },
    },
};

const ATTACH_RIGHTS: PROCESS_ACCESS_RIGHTS = PROCESS_ACCESS_RIGHTS(
    PROCESS_CREATE_THREAD.0
        | PROCESS_QUERY_INFORMATION.0
        | PROCESS_VM_OPERATION.0
        | PROCESS_VM_READ.0
        | PROCESS_VM_WRITE.0
        | PROCESS_DUP_HANDLE.0,
);

pub struct Process {
    #[allow(unused)]
    pid: u32,
    handle: HANDLE,
}

impl Process {
    #[allow(unused)]
    pub fn new(process: &sysinfo::Process) -> Process {
        Self::try_new(process).unwrap()
    }

    pub fn try_new(process: &sysinfo::Process) -> Result<Process, windows::core::Error> {
        let pid = process.pid().as_u32();
        Self::try_from_pid(pid)
    }

    pub fn current_process() -> Process {
        let pid = std::process::id();

        Self::try_from_pid(pid).unwrap()
    }

    pub fn try_from_pid(pid: u32) -> Result<Process, windows::core::Error> {
        let process_handle = unsafe { OpenProcess(ATTACH_RIGHTS, false, pid) }?;

        Ok(Process {
            pid,
            handle: process_handle,
        })
    }

    pub fn iter_modules(&self, mut iter: impl FnMut(&OsStr)) -> Result<(), windows::core::Error> {
        const LEN: usize = 1024;

        let mut needed = 0;

        unsafe {
            EnumProcessModulesEx(
                self.handle,
                null_mut(),
                0,
                &mut needed,
                ENUM_PROCESS_MODULES_EX_FLAGS(3),
            )?
        };

        let needed = needed as usize;

        let slice: &mut [HMODULE] = if needed > LEN * size_of::<HMODULE>() {
            &mut Vec::with_capacity(needed)
        } else {
            &mut [HMODULE(null_mut()); LEN]
        };

        let slice_ptr = slice.as_mut_ptr();
        let mut new_needed = 0;

        unsafe {
            EnumProcessModulesEx(
                self.handle,
                slice_ptr,
                size_of_val(slice) as u32,
                &mut new_needed,
                ENUM_PROCESS_MODULES_EX_FLAGS(3),
            )?
        }

        assert_eq!(needed, new_needed as usize);

        let len = new_needed as usize / size_of::<HMODULE>();
        slice[0..len].iter().try_for_each(|module| {
            let mut name_bfr = [0; MAX_PATH as usize];
            let len =
                unsafe { GetModuleFileNameExW(Some(self.handle), Some(*module), &mut name_bfr) };

            if len == 0 {
                Err(windows::core::Error::from_win32())
            } else {
                use std::os::windows::prelude::*;

                let ostr = OsString::from_wide(&name_bfr[..len as usize]);

                iter(&ostr);

                Ok(())
            }
        })
    }

    #[allow(unused)]
    pub fn pid(&self) -> u32 {
        self.pid
    }

    #[allow(unused)]
    pub fn handle(&self) -> HANDLE {
        self.handle
    }

    #[allow(unused)]
    pub unsafe fn duplicate_handle(
        &self,
        target_process: &Process,
        original_shared_handle: HANDLE,
        shared_rights: u32,
    ) -> Result<HANDLE, windows::core::Error> {
        let mut shared_handle = HANDLE::default();

        unsafe {
            DuplicateHandle(
                self.handle,
                original_shared_handle,
                target_process.handle,
                &mut shared_handle,
                shared_rights,
                false,
                DUPLICATE_HANDLE_OPTIONS::default(),
            )
        }?;

        Ok(shared_handle)
    }

    pub fn is_64_bit(&self) -> bool {
        let mut is_64_bit = windows::core::BOOL::default();

        unsafe { IsWow64Process(self.handle, &raw mut is_64_bit).unwrap() };

        !is_64_bit.as_bool()
    }

    pub unsafe fn load_remote_library(
        &self,
        library_path: &Path,
    ) -> Result<(), windows::core::Error> {
        // TODO: These dlls should be included, written to a temp file, and run from that instead
        let program = if self.is_64_bit() {
            "./load_library_getter_64.exe"
        } else {
            "./load_library_getter_32.exe"
        };

        let load_library_ptr = std::process::Command::new(program)
            .stdout(Stdio::piped())
            .output()
            .unwrap();
        println!("Getter: {load_library_ptr:?}");
        let load_library_ptr: usize = String::from_utf8(load_library_ptr.stdout)
            .unwrap()
            .parse()
            .unwrap();
        println!("Getter: {}", load_library_ptr);

        // Encode it as null terminated UTF-16
        let utf_16 = os_str_to_pcwstr(library_path.as_os_str());
        // This means that the size of the alloc is size_of::<u16> * length of slice
        let alloc_size = utf_16.len() * size_of::<u16>();

        // So we allocate that
        let virtual_alloc = unsafe {
            VirtualAllocEx(
                self.handle,
                None,
                alloc_size,
                MEM_COMMIT | MEM_RESERVE,
                PAGE_READWRITE,
            )
        };

        assert_ne!(virtual_alloc, null_mut());

        // And write it
        unsafe {
            WriteProcessMemory(
                self.handle,
                virtual_alloc,
                utf_16.as_ptr() as _,
                alloc_size,
                None,
            )?;
        }

        let load_library_ptr: unsafe extern "system" fn(*mut std::ffi::c_void) -> u32 =
            unsafe { std::mem::transmute(load_library_ptr) };

        unsafe {
            CreateRemoteThread(
                self.handle,
                None,
                0,
                Some(load_library_ptr),
                Some(virtual_alloc),
                0,
                None,
            )?;
        }

        Ok(())
    }
}

fn os_str_to_pcwstr(os_str: &OsStr) -> Box<[u16]> {
    use std::os::windows::ffi::OsStrExt;
    let mut utf16: Vec<u16> = os_str.encode_wide().collect();
    // Add a null byte at the end
    utf16.push(0x0);
    utf16.into_boxed_slice()
}
