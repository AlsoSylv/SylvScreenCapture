use std::{
    ptr::NonNull,
    sync::atomic::{AtomicI32, AtomicU32, AtomicU64, AtomicU8},
};

pub use shared_memory::ShmemError;
use windows::Win32::Foundation::HANDLE;

const HEADER_SIZE: usize = size_of::<SharedMemoryHeader>();

pub struct ShmemBuilder {
    inner: shared_memory::ShmemConf,
}

impl ShmemBuilder {
    /// Automatically allocates the size of the header
    pub fn new(name: impl AsRef<str>) -> ShmemBuilder {
        let inner = shared_memory::ShmemConf::new()
            .os_id(name)
            .size(HEADER_SIZE);

        Self { inner }
    }

    /// This automatically allocs size + size_of::<SharedMemoryHeader>
    pub fn size(self, size: usize) -> ShmemBuilder {
        Self {
            inner: self.inner.size(size + HEADER_SIZE),
        }
    }

    pub fn create(self) -> Result<Shmem, ShmemError> {
        let inner = self.inner.create()?;
        let outer = Shmem { inner };
        outer.init_header();
        Ok(outer)
    }

    pub fn open(self) -> Result<Shmem, ShmemError> {
        let inner = self.inner.open()?;
        Ok(Shmem { inner })
    }
}

pub struct Shmem {
    inner: shared_memory::Shmem,
}

impl Shmem {
    fn header_ptr(&self) -> *mut SharedMemoryHeader {
        self.inner.as_ptr() as _
    }

    fn init_header(&self) {
        let header = self.header_ptr();
        unsafe { *header = Default::default() }
    }

    pub fn header(&self) -> &SharedMemoryHeader {
        unsafe { &*self.header_ptr() }
    }

    pub fn buffer(&self) -> &[u8] {
        let ptr = self.buffer_ptr();

        unsafe { std::slice::from_raw_parts(ptr, self.buffer_size()) }
    }

    pub fn buffer_ptr(&self) -> *mut u8 {
        unsafe { self.inner.as_ptr().add(HEADER_SIZE) }
    }

    pub fn buffer_size(&self) -> usize {
        self.inner.len() - HEADER_SIZE
    }

    pub fn set_owner(&mut self) {
        self.inner.set_owner(true);
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
            as isize as *mut std::ffi::c_void);
        handle.map(|ptr| HANDLE(ptr.as_ptr() as _))
    }

    pub fn set_nt_shared_handle(&self, handle: *mut std::ffi::c_void) {
        self.nt_shared_handle
            .store(handle as _, std::sync::atomic::Ordering::SeqCst);
    }

    pub fn get_nt_shared_handle(&self) -> Option<HANDLE> {
        let handle = NonNull::new(self.shared_handle.load(std::sync::atomic::Ordering::SeqCst)
            as isize as *mut std::ffi::c_void);
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

    pub fn flip(&self) -> bool {
        matches!(self.api(), RenderingAPI::Ogl)
    }

    pub fn ignore_alpha(&self) -> bool {
        matches!(self.api(), RenderingAPI::Dx9 | RenderingAPI::Dx9x)
    }

    pub fn nt_handle_in_use(&self) -> bool {
        matches!(
            self.api(),
            RenderingAPI::Dx11 | RenderingAPI::Dx12 | RenderingAPI::Ogl | RenderingAPI::Vk
        )
    }
}
