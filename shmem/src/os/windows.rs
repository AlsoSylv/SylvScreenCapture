use std::ffi::{CStr, c_void};

use windows::{
    Win32::{
        Foundation::{CloseHandle, HANDLE},
        System::Memory::{
            CreateFileMappingA, FILE_MAP_ALL_ACCESS, MEMORY_MAPPED_VIEW_ADDRESS, MapViewOfFile,
            OpenFileMappingA, PAGE_READWRITE, UnmapViewOfFile,
        },
    },
    core::{Error, PCSTR},
};

use super::common::View;

pub struct ShMem<'a, T>
where
    T: Default,
{
    file_mapping: HANDLE,
    view: &'a mut View<T>,
}

impl<'a, T> ShMem<'a, T>
where
    T: Default,
{
    pub fn new(name: &CStr) -> Result<Self, Error> {
        Self::create_or_open(name, true)
    }

    pub fn open(name: &CStr) -> Result<Self, Error> {
        Self::create_or_open(name, false)
    }

    fn create_or_open(name: &CStr, create: bool) -> Result<ShMem<'a, T>, Error> {
        let file_mapping = if create {
            let size = size_of::<View<T>>();
            let high = (size >> 32) as u32;
            let low = size as u32;

            // SAFETY: The mapped file must be the length of the View<T> type and needs to be created here
            unsafe {
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
            // SAFETY: The opened mapping should have a unique id so that it does not overlap with anything else
            // Read and write access is allowed
            unsafe {
                OpenFileMappingA(
                    FILE_MAP_ALL_ACCESS.0,
                    false,
                    PCSTR::from_raw(name.as_ptr() as _),
                )?
            }
        };

        let view = {
            // SAFETY: The size of the view is the same as the map, and it is a view into the `View<T>` struct
            let address = unsafe {
                MapViewOfFile(
                    file_mapping,
                    FILE_MAP_ALL_ACCESS,
                    0,
                    0,
                    size_of::<View<T>>(),
                )
            };

            // This means that the view failed
            if address.Value.is_null() {
                return Err(Error::from_win32());
            }

            let view = address.Value as *mut View<T>;
            if create {
                // SAFETY: The type of the pointer has not been erased at any step
                // But because the View COULD be in an invalid state, it must be set to the default
                unsafe { view.write(View::new()) };
            }

            // SAFETY: This is a non-null, valid object, which lasts for `'_`, and as such, is entirely within the safety of a rust reference
            unsafe { &mut *view }
        };

        view.inc_ref_count();

        Ok(Self { file_mapping, view })
    }

    pub fn as_ref(&self) -> &T {
        self.view.as_ref()
    }

    pub fn as_mut(&mut self) -> &mut T {
        self.view.as_mut()
    }

    /// #Safety
    /// Calling this can trigger drop to be called
    /// This should only ever be called ONCE per program, either on shutdown or when the memory is no longer in use
    pub unsafe fn dec_ref_count(&mut self) {
        let prev = self.view.dec_ref_count();

        unsafe {
            // SAFETY: This is part of the fact that this function can only be called when the object is being destoryed
            UnmapViewOfFile(MEMORY_MAPPED_VIEW_ADDRESS {
                Value: self.view as *mut _ as *mut c_void,
            })
            .unwrap();
        }

        // The refcount is now 0
        if prev == 1 {
            // SAFETY: If this is the last program referencing the memory, then it should be closed
            unsafe { CloseHandle(self.file_mapping).unwrap() };
        }
    }
}

impl<T> Drop for ShMem<'_, T>
where
    T: Default,
{
    fn drop(&mut self) {
        unsafe { self.dec_ref_count() };
    }
}
