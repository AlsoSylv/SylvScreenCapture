use std::{ffi::OsString, ptr::null_mut};

use sysinfo::{ProcessRefreshKind, RefreshKind, System};
use windows::Win32::{
    Foundation::{HANDLE, HMODULE, MAX_PATH},
    System::{
        ProcessStatus::{
            EnumProcessModulesEx, GetModuleFileNameExW, ENUM_PROCESS_MODULES_EX_FLAGS,
        },
        Threading::{
            OpenProcess, PROCESS_ACCESS_RIGHTS, PROCESS_CREATE_THREAD, PROCESS_DUP_HANDLE,
            PROCESS_QUERY_INFORMATION, PROCESS_VM_OPERATION, PROCESS_VM_READ, PROCESS_VM_WRITE,
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
    pub handle: HANDLE,
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
}
