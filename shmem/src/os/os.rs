use std::{
    ffi::{c_void, CStr},
    marker::PhantomData,
    ptr::NonNull,
};

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
    view: NonNull<View<T>>,
    lifetime: PhantomData<&'a mut T>,
}

impl<'a, T> ShMem<'a, T> {
    pub fn new(name: &CStr) -> Result<Self, Error> {
        let size = size_of::<View<T>>();
        let high = ((size >> 32) & 0xFFFFFFFF) as u32;
        let low = (size & 0xFFFFFFFF) as u32;

        let mapping = unsafe {
            CreateFileMappingA(
                None,
                None,
                PAGE_READWRITE,
                high,
                low,
                PCSTR::from_raw(name.as_ptr() as _),
            )?
        };

        let view = unsafe {
            let address = MapViewOfFile(mapping, FILE_MAP_ALL_ACCESS, 0, 0, size_of::<View<T>>());

            NonNull::new(address.Value as *mut View<T>).unwrap()
        };

        unsafe {
            view.as_ref()
                .ref_count()
                .fetch_add(1, std::sync::atomic::Ordering::SeqCst)
        };

        Ok(Self {
            file_mapping: mapping,
            view,
            lifetime: PhantomData,
        })
    }

    pub fn open(name: &CStr) -> Result<Self, Error> {
        let mapping = unsafe {
            OpenFileMappingA(
                FILE_MAP_ALL_ACCESS.0,
                false,
                PCSTR::from_raw(name.as_ptr() as _),
            )?
        };

        let view = unsafe {
            let address = MapViewOfFile(mapping, FILE_MAP_ALL_ACCESS, 0, 0, size_of::<View<T>>());

            NonNull::new(address.Value as *mut View<T>).unwrap()
        };

        unsafe {
            view.as_ref()
                .ref_count()
                .fetch_add(1, std::sync::atomic::Ordering::SeqCst)
        };

        Ok(Self {
            file_mapping: mapping,
            view,
            lifetime: PhantomData,
        })
    }

    pub fn as_ref(&self) -> &T {
        unsafe { self.view.as_ref().as_ref() }
    }

    pub fn as_mut(&mut self) -> &mut T {
        unsafe { self.view.as_mut().as_mut() }
    }
}

impl<'a, T> Drop for ShMem<'a, T> {
    fn drop(&mut self) {
        let view = unsafe { self.view.as_ref() }.ref_count();
        let prev = view.fetch_sub(1, std::sync::atomic::Ordering::SeqCst);

        unsafe {
            UnmapViewOfFile(MEMORY_MAPPED_VIEW_ADDRESS {
                Value: self.view.as_mut() as *mut _ as *mut c_void,
            })
            .unwrap();
        }

        // The refcount is now 0
        if prev == 1 {
            unsafe { CloseHandle(self.file_mapping).unwrap() };
        }
    }
}
