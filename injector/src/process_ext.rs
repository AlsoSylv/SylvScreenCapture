use std::{
    ffi::{OsStr, OsString},
    path::Path,
    ptr::null_mut,
};

use sysinfo::{ProcessRefreshKind, RefreshKind, System};
use windows::{
    core::{s, w},
    Win32::{
        Foundation::{DuplicateHandle, DUPLICATE_HANDLE_OPTIONS, HANDLE, HMODULE, MAX_PATH},
        System::{
            Diagnostics::Debug::WriteProcessMemory,
            LibraryLoader::{GetModuleHandleW, GetProcAddress},
            Memory::{VirtualAllocEx, MEM_COMMIT, MEM_RESERVE, PAGE_READWRITE},
            ProcessStatus::{
                EnumProcessModulesEx, GetModuleFileNameExW, ENUM_PROCESS_MODULES_EX_FLAGS,
            },
            Threading::{
                CreateRemoteThread, OpenProcess, PROCESS_ACCESS_RIGHTS, PROCESS_CREATE_THREAD,
                PROCESS_DUP_HANDLE, PROCESS_QUERY_INFORMATION, PROCESS_VM_OPERATION,
                PROCESS_VM_READ, PROCESS_VM_WRITE,
            },
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
    pub fn new(name: &str) -> Process {
        Self::try_new(name).unwrap()
    }

    pub fn new_with_system(name: &str, system: &System) -> Process {
        Self::try_new_with_system(name, system).unwrap()
    }

    pub fn try_new(name: &str) -> Result<Process, windows::core::Error> {
        let system = System::new_with_specifics(
            RefreshKind::new().with_processes(ProcessRefreshKind::new()),
        );

        Self::try_new_with_system(name, &system)
    }

    pub fn try_new_with_system(
        name: &str,
        system: &System,
    ) -> Result<Process, windows::core::Error> {
        let process = system.processes_by_name(name.as_ref()).next().unwrap();
        let pid = process.pid().as_u32();
        let process_handle = unsafe { OpenProcess(ATTACH_RIGHTS, false, pid)? };

        Ok(Process {
            pid,
            handle: process_handle,
        })
    }

    pub fn current_process() -> Process {
        let pid = std::process::id();
        let process_handle = unsafe { OpenProcess(ATTACH_RIGHTS, false, pid) }.unwrap();

        Process {
            pid,
            handle: process_handle,
        }
    }

    pub fn get_modules(&self) -> Result<Vec<OsString>, windows::core::Error> {
        const LEN: usize = 1024;

        let mut needed = 0;

        unsafe {
            EnumProcessModulesEx(
                self.handle,
                null_mut(),
                0,
                &mut needed,
                ENUM_PROCESS_MODULES_EX_FLAGS(0),
            )?
        };

        let needed = needed as usize;

        let slice: &mut [HMODULE] = if needed > LEN * size_of::<HMODULE>() {
            &mut Vec::with_capacity(needed)
        } else {
            &mut [HMODULE(null_mut()); LEN as usize]
        };

        let slice_ptr = slice.as_mut_ptr();
        let slice_len = slice.len();
        let mut new_needed = 0;

        unsafe {
            EnumProcessModulesEx(
                self.handle,
                slice_ptr,
                (slice_len * size_of::<HMODULE>()) as u32,
                &mut new_needed,
                ENUM_PROCESS_MODULES_EX_FLAGS(0),
            )?
        }

        assert_eq!(needed, new_needed as usize);

        let len = new_needed as usize / size_of::<HMODULE>();
        slice[0..len]
            .iter()
            .map(|module| {
                let mut name_bfr = [0; MAX_PATH as usize];
                let len = unsafe { GetModuleFileNameExW(self.handle, *module, &mut name_bfr) };

                if len == 0 {
                    Err(windows::core::Error::from_win32())
                } else {
                    use std::os::windows::prelude::*;
                    Ok(OsString::from_wide(&name_bfr[..len as usize]))
                }
            })
            .collect()
    }

    #[allow(unused)]
    pub fn pid(&self) -> u32 {
        self.pid
    }

    pub fn handle(&self) -> HANDLE {
        self.handle
    }

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

    pub unsafe fn load_remote_library(
        &self,
        library_path: &Path,
    ) -> Result<(), windows::core::Error> {
        const KERNEL_32_DLL: windows::core::PCWSTR = w!("kernel32.dll");
        const LOAD_LIBRARY_A_C: windows::core::PCSTR = s!("LoadLibraryW");

        let module = unsafe { GetModuleHandleW(KERNEL_32_DLL) }?;
        let load_library_ptr = unsafe { GetProcAddress(module, LOAD_LIBRARY_A_C) }
            .expect("kernel32.dll always contains LoadLibraryW");
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

        unsafe {
            CreateRemoteThread(
                self.handle,
                None,
                0,
                Some(std::mem::transmute(load_library_ptr)),
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
