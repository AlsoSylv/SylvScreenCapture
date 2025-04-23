use std::{
    ffi::CStr,
    ptr::NonNull,
    sync::atomic::{AtomicI32, AtomicU32, AtomicU64, AtomicU8},
};

use windows::Win32::Foundation::HANDLE;

mod os;

pub struct Shmem<'a, T> {
    inner: os::ShMem<'a, T>,
}

// In theory, as long as the inner type would be safe across multiple threads, the shared memory is
unsafe impl<'a, T> Send for Shmem<'a, T> where T: Send {}
unsafe impl<'a, T> Sync for Shmem<'a, T> where T: Sync {}

impl<'a, T> Shmem<'a, T> {
    pub fn new(name: &CStr) -> Self {
        Shmem {
            inner: os::ShMem::new(name).unwrap(),
        }
    }

    pub fn open(name: &CStr) -> Self {
        Shmem {
            inner: os::ShMem::open(name).unwrap(),
        }
    }

    pub fn as_ref(&self) -> &T {
        self.inner.as_ref()
    }

    pub fn as_mut(&mut self) -> &mut T {
        self.inner.as_mut()
    }

    pub unsafe fn dec_ref_count(&self) {
        unsafe {
            self.inner.dec_ref_count();
        }
    }
}

#[derive(Default, Debug, PartialEq)]
#[repr(u8)]
pub enum RenderingAPI {
    #[default]
    None = 0b0000,
    Ogl = 0b0001,
    Vk = 0b0011,
    Dx7 = 0b0111, // The other unloved child
    Dx8 = 0b1111, // The unloved child
    Dx9 = 0b1110,
    Dx9x = 0b1100,
    Dx10 = 0b1000,
    Dx11 = 0b1101,
    Dx12 = 0b1011,
}

impl RenderingAPI {
    pub fn flip(&self) -> bool {
        matches!(self, RenderingAPI::Ogl)
    }

    pub fn ignore_alpha(&self) -> bool {
        matches!(self, RenderingAPI::Dx9 | RenderingAPI::Dx9x)
    }

    pub fn nt_handle_in_use(&self) -> bool {
        matches!(
            self,
            RenderingAPI::Dx11 | RenderingAPI::Dx12 | RenderingAPI::Ogl | RenderingAPI::Vk
        )
    }
}

impl TryFrom<u8> for RenderingAPI {
    type Error = u8;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        use RenderingAPI::*;

        match value {
            int if int == None as u8 => Ok(None),
            int if int == Ogl as u8 => Ok(Ogl),
            int if int == Vk as u8 => Ok(Vk),
            int if int == Dx7 as u8 => Ok(Dx7),
            int if int == Dx8 as u8 => Ok(Dx8),
            int if int == Dx9 as u8 => Ok(Dx9),
            int if int == Dx9x as u8 => Ok(Dx9x),
            int if int == Dx10 as u8 => Ok(Dx10),
            int if int == Dx11 as u8 => Ok(Dx11),
            int if int == Dx12 as u8 => Ok(Dx12),
            int => Err(int),
        }
    }
}

#[derive(Default)]
#[repr(C)]
pub struct SharedMemoryHeader {
    /// hi: width: u32, lo: height: u32
    dimensions: AtomicU64,
    /// shared handle to the D3D NT Handle
    nt_shared_handle: AtomicI32,
    /// shared handle to the D3D Handle
    shared_handle: AtomicI32,
    pid: AtomicU32,
    /**
    None = 0b0000,
    Ogl  = 0b0001,
    Vk   = 0b0011,
    Dx7  = 0b0111, // The other unloved child
    Dx8  = 0b1111, // The unloved child
    Dx9  = 0b1110,
    Dx9x = 0b1100,
    Dx10 = 0b1000,
    Dx11 = 0b1101,
    Dx12 = 0b1011,
     **/
    api: AtomicU8,
    // Three bytes left
}

const _: () = const { assert!(size_of::<SharedMemoryHeader>() == 24) };

impl SharedMemoryHeader {
    pub fn set_shared_handle(&self, handle: *mut std::ffi::c_void) {
        self.shared_handle
            .store(handle as _, std::sync::atomic::Ordering::SeqCst);
    }

    pub fn get_shared_handle(&self) -> Option<HANDLE> {
        let handle = NonNull::new(self.shared_handle.load(std::sync::atomic::Ordering::SeqCst)
            as u32 as usize as isize as *mut std::ffi::c_void);
        handle.map(|ptr| HANDLE(ptr.as_ptr() as _))
    }

    pub fn set_nt_shared_handle(&self, handle: *mut std::ffi::c_void) {
        self.nt_shared_handle
            .store(handle as _, std::sync::atomic::Ordering::SeqCst);
    }

    pub fn get_nt_shared_handle(&self) -> Option<HANDLE> {
        let handle = NonNull::new(
            self.nt_shared_handle
                .load(std::sync::atomic::Ordering::SeqCst) as isize
                as *mut std::ffi::c_void,
        );
        handle.map(|ptr| HANDLE(ptr.as_ptr() as _))
    }

    pub fn get_width_and_height(&self) -> (u32, u32) {
        let dimensions = self.dimensions.load(std::sync::atomic::Ordering::SeqCst);
        ((dimensions >> 32) as _, dimensions as _)
    }

    pub fn set_width_and_height(&self, width: u32, height: u32) {
        let width = (width as u64) << 32;
        let height = height as u64;
        let dimensions = width | height;
        self.dimensions
            .store(dimensions, std::sync::atomic::Ordering::SeqCst);
    }

    pub fn set_pid(&self) {
        self.pid
            .store(std::process::id(), std::sync::atomic::Ordering::SeqCst);
    }

    pub fn api(&self) -> RenderingAPI {
        let api = self.api.load(std::sync::atomic::Ordering::SeqCst);
        api.try_into().unwrap()
    }

    pub fn set_api(&self, api: RenderingAPI) {
        self.api
            .store(api as u8, std::sync::atomic::Ordering::SeqCst);
    }
}
