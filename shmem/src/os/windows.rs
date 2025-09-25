use core::{
    ffi::CStr,
    ops::{Deref, DerefMut},
    ptr::NonNull,
};

use windows::{
    Win32::{
        Foundation::{CloseHandle, GetLastError, HANDLE, WAIT_EVENT},
        Storage::FileSystem::SYNCHRONIZE,
        System::{
            Memory::{
                CreateFileMappingA, FILE_MAP_ALL_ACCESS, MEMORY_MAPPED_VIEW_ADDRESS, MapViewOfFile,
                OpenFileMappingA, PAGE_READWRITE, UnmapViewOfFile,
            },
            Threading::{CreateMutexA, INFINITE, ReleaseMutex, WaitForSingleObject},
            WindowsProgramming::OpenMutexA,
        },
    },
    core::{Error, PCSTR},
};

use super::common::View;

type ViewPtr<T> = NonNull<View<T>>;

#[repr(C)]
pub struct ShMem<T>
where
    T: Default,
{
    file_mapping: HANDLE,
    view: Option<ViewPtr<T>>,
}

impl<T> ShMem<T>
where
    T: Default,
{
    pub fn new(name: &CStr) -> Result<Self, Error> {
        Self::create_or_open(name, true)
    }

    pub fn open(name: &CStr) -> Result<Self, Error> {
        Self::create_or_open(name, false)
    }

    fn create_or_open(name: &CStr, create: bool) -> Result<Self, Error> {
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

        let Some(view): Option<ViewPtr<T>> = NonNull::new(address.Value as _) else {
            return Err(Error::from_thread());
        };

        if create {
            // SAFETY: The type of the pointer has not been erased at any step
            // But because the View COULD be in an invalid state, it must be set to the default
            unsafe { view.write(View::new()) };
        }

        unsafe { view.as_ref().inc_ref_count() };

        Ok(Self {
            file_mapping,
            view: Some(view),
        })
    }

    pub fn view(&self) -> &View<T> {
        unsafe { &*self.view.unwrap().as_ref() }
    }

    pub fn view_mut(&mut self) -> &mut View<T> {
        unsafe { &mut *self.view.unwrap().as_mut() }
    }

    /// # Safety
    /// Calling this can trigger drop to be called
    /// This should only ever be called ONCE per program, either on shutdown or when the memory is no longer in use
    pub unsafe fn dec_program_count(&mut self) {
        let _ = self.view().dec_ref_count();

        unsafe {
            // SAFETY: The view is owned by the currnet process, and is not shared
            UnmapViewOfFile(MEMORY_MAPPED_VIEW_ADDRESS {
                Value: self.view_mut() as *mut _ as _,
            })
            .unwrap();
        }

        self.view = None;

        unsafe { CloseHandle(self.file_mapping).unwrap() };
        self.file_mapping = HANDLE::default();
    }
}

impl<T> Deref for ShMem<T>
where
    T: Default,
{
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.view()
    }
}

impl<T> DerefMut for ShMem<T>
where
    T: Default,
{
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.view_mut()
    }
}

impl<T> Drop for ShMem<T>
where
    T: Default,
{
    fn drop(&mut self) {
        unsafe { self.dec_program_count() };
    }
}

pub struct Mutex {
    mutex: HANDLE,
}

impl Mutex {
    pub fn new(name: &CStr) -> Result<Self, windows::core::Error> {
        let mutex = unsafe { CreateMutexA(None, false, PCSTR(name.as_ptr() as _)) }?;

        Ok(Self { mutex })
    }

    pub fn open(name: &CStr) -> Result<Self, windows::core::Error> {
        let mutex = unsafe { OpenMutexA(SYNCHRONIZE.0, false, PCSTR(name.as_ptr() as _)) };

        if mutex.is_invalid() {
            return unsafe { Err(GetLastError())? };
        }

        Ok(Self { mutex })
    }

    pub fn lock(&self) -> WAIT_EVENT {
        unsafe { WaitForSingleObject(self.mutex, INFINITE) }
    }

    pub fn release(&self) -> Result<(), windows::core::Error> {
        unsafe { ReleaseMutex(self.mutex) }
    }
}
