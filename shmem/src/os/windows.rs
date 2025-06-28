use std::{cell::UnsafeCell, ffi::CStr, marker::PhantomData};

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

type ViewPtr<'a, T> = &'a UnsafeCell<View<T>>;

pub struct ShMem<'a, T>
where
    T: Default,
{
    file_mapping: HANDLE,
    view: Option<ViewPtr<'a, T>>,
    lifetime: PhantomData<&'a T>,
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
            #[cfg(target_pointer_width = "64")]
            let high = (size >> 32) as u32;
            #[cfg(target_pointer_width = "32")]
            let high = 0;
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
                    size_of::<UnsafeCell<View<T>>>(),
                )
            };

            // This means that the view failed
            if address.Value.is_null() {
                return Err(Error::from_win32());
            }

            let view = address.Value as *mut UnsafeCell<View<T>>;
            if create {
                // SAFETY: The type of the pointer has not been erased at any step
                // But because the View COULD be in an invalid state, it must be set to the default
                unsafe { view.write(UnsafeCell::new(View::new())) };
            }

            // SAFETY: This was initialized above
            unsafe { &*view }
        };

        unsafe { &*view.get() }.inc_ref_count();

        Ok(Self {
            file_mapping,
            view: Some(view),
            lifetime: PhantomData,
        })
    }

    pub fn as_ref(&self) -> &T {
        self.view().as_ref()
    }

    pub fn view(&self) -> &View<T> {
        unsafe { &*self.view.unwrap().get() }
    }

    /// # Safety
    /// Calling this can trigger drop to be called
    /// This should only ever be called ONCE per program, either on shutdown or when the memory is no longer in use
    pub unsafe fn dec_ref_count(&mut self) {
        let prev = self.view().dec_ref_count();

        unsafe {
            // SAFETY: The view is owned by the currnet process, and is not shared
            UnmapViewOfFile(MEMORY_MAPPED_VIEW_ADDRESS {
                Value: self.view.unwrap().get() as _,
            })
            .unwrap();
        }

        self.view = None;

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
