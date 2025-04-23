use std::ffi::{c_void, CStr};

use windows::{
    core::{Error, PCSTR},
    Win32::{
        Foundation::{CloseHandle, HANDLE},
        System::Memory::{
            CreateFileMappingA, MapViewOfFile, OpenFileMappingA, UnmapViewOfFile,
            FILE_MAP_ALL_ACCESS, MEMORY_MAPPED_VIEW_ADDRESS, PAGE_READWRITE,
        },
    },
};

use super::common::View;

pub struct ShMem<'a, T> {
    file_mapping: HANDLE,
    view: &'a mut View<T>,
}

impl<'a, T> ShMem<'a, T> {
    pub fn new(name: &CStr) -> Result<Self, Error> {
        Self::create_or_open(name, true)
    }

    pub fn open(name: &CStr) -> Result<Self, Error> {
        Self::create_or_open(name, false)
    }

    fn create_or_open(name: &CStr, create: bool) -> Result<ShMem<'a, T>, Error> {
        let file_mapping = if create {
            unsafe {
                let size = size_of::<View<T>>();
                let high = ((size >> 32) & 0xFFFFFFFF) as u32;
                let low = (size & 0xFFFFFFFF) as u32;

                CreateFileMappingA(
                    HANDLE::default(),
                    None,
                    PAGE_READWRITE,
                    high,
                    low,
                    PCSTR::from_raw(name.as_ptr() as _),
                )?
            }
        } else {
            unsafe {
                OpenFileMappingA(
                    FILE_MAP_ALL_ACCESS.0,
                    false,
                    PCSTR::from_raw(name.as_ptr() as _),
                )?
            }
        };

        let view = unsafe {
            let address = MapViewOfFile(
                file_mapping,
                FILE_MAP_ALL_ACCESS,
                0,
                0,
                size_of::<View<T>>(),
            );

            if address.Value.is_null() {
                return Err(Error::from_win32());
            }

            &mut *(address.Value as *mut View<T>)
        };

        view.ref_count()
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst);

        Ok(Self { file_mapping, view })
    }

    pub fn as_ref(&self) -> &T {
        self.view.as_ref()
    }

    pub fn as_mut(&mut self) -> &mut T {
        self.view.as_mut()
    }

    /// SAFETY: Calling this can trigger drop to be called
    pub unsafe fn dec_ref_count(&mut self) {
        let prev = self.view.dec_ref_count();

        unsafe {
            UnmapViewOfFile(MEMORY_MAPPED_VIEW_ADDRESS {
                Value: self.view as *mut _ as *mut c_void,
            })
            .unwrap();
        }

        // The refcount is now 0
        if prev == 1 {
            unsafe { CloseHandle(self.file_mapping).unwrap() };
        }
    }
}

impl<'a, T> Drop for ShMem<'a, T> {
    fn drop(&mut self) {
        unsafe { self.dec_ref_count() };
    }
}
