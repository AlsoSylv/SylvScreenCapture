// use interprocess::local_socket::traits::Stream as StreamTrait;
// use interprocess::local_socket::{GenericNamespaced, Stream, ToNsName};
use retour::{Function, RawDetour};
use std::ffi::c_void;
// use std::io::{ErrorKind, Read, Write};
use std::ptr::NonNull;
use std::sync::atomic::{AtomicPtr, AtomicU32, AtomicU64, AtomicU8};
use std::sync::OnceLock;
use windows::Win32::Foundation::{HANDLE, HMODULE};
use windows::Win32::System::SystemServices;
use windows::{
    core::{s, PCSTR},
    Win32::{
        Foundation::{BOOL, HINSTANCE},
        System::{
            Console::AllocConsole,
            LibraryLoader::{DisableThreadLibraryCalls, GetModuleHandleA},
        },
    },
};

use error::Error;

mod error;
mod impls;

#[repr(u8)]
enum InUseRenderingAPI {
    Ogl = 0b000,
    Vk = 0b001,
    Dx8 = 0b101, // The unloved child
    Dx9 = 0b010,
    Dx9x = 0b100,
    Dx10 = 0b011,
    Dx11 = 0b110,
    Dx12 = 0b111,
}

impl TryFrom<u8> for InUseRenderingAPI {
    type Error = u8;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        use InUseRenderingAPI::*;

        match value {
            int if int == Ogl as u8 => Ok(Ogl),
            int if int == Vk as u8 => Ok(Vk),
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

#[repr(C)]
struct NewSharedMemoryHeader {
    /// shared handle to the D3D NT Handle
    shared_handle: AtomicPtr<c_void>,
    /// hi: width: u32, lo: height: u32
    dimensions: AtomicU64,
    pid: AtomicU32,
    /**
     * OGL =  000;
     * VK  =  001;
     * DX9 =  010;
     * DX9E = 100;
     * DX10 = 011;
     * DX11 = 110;
     * DX12 = 111;
     **/
    api: AtomicU8,
}

impl NewSharedMemoryHeader {
    fn get_width_and_height(&self) -> (u32, u32) {
        let dimensions = self.dimensions.load(std::sync::atomic::Ordering::SeqCst);
        ((dimensions >> 32) as _, dimensions as _)
    }

    fn set_width_and_height(&self, width: u32, height: u32) {
        let width = (width as u64) << 32;
        let height = height as u64;
        let dimensions = width | height;
        self.dimensions
            .store(dimensions, std::sync::atomic::Ordering::SeqCst);
    }

    fn set_pid(&self) {
        self.pid
            .store(std::process::id(), std::sync::atomic::Ordering::SeqCst);
    }

    fn get_shared_handle(&self) -> Option<HANDLE> {
        let handle = NonNull::new(self.shared_handle.load(std::sync::atomic::Ordering::SeqCst));
        handle.map(|ptr| HANDLE(ptr.as_ptr()))
    }

    fn set_api(&self, api: InUseRenderingAPI) {
        let api = self
            .api
            .store(api as u8, std::sync::atomic::Ordering::SeqCst);
        api.try_into().unwrap()
    }
}

pub trait RenderingAPI: Sized {
    type PresentFn: Function;
    type ResizeFn: Function;

    fn present_fn(&self) -> *const ();

    fn resize_fn(&self) -> *const ();

    fn create(module: HMODULE) -> Result<Self, Error>;

    fn destroy(&self) -> Result<(), Error>;

    fn trampoline() -> Self::PresentFn;

    fn set_trampoline(func: Self::PresentFn);

    fn new_present_fn() -> Self::PresentFn;

    fn set_detour(detour: RawDetour);
}

#[derive(PartialEq)]
enum Reason {
    DllProcessAttach,
    DllProcessDetach,
}

// TODO: Implement a clearer shared memory layout
/*
    The ideal layout in my head is
    struct SharedMemory {
        shared_handle: AtomicU64,
        dimensions: AtomicU64, (hi: width: u32, lo: height: u32)
        api: AtomicU8,
        flip: AtomicBool,
        ignore_alpha: AtomicBool,
    }
*/
#[repr(transparent)]
pub struct SharedMem(pub shared_memory::Shmem);

unsafe impl Send for SharedMem {}
unsafe impl Sync for SharedMem {}

pub static SHARED_CPU_BUFFER: OnceLock<SharedMem> = OnceLock::new();
// pub static SHARED_HANDLE: AtomicPtr<c_void> = AtomicPtr::new(null_mut());

// Export this main as DllMain
#[export_name = "DllMain"]
pub extern "stdcall" fn dll_main(hinst_dll: HINSTANCE, fdw_reason: u32, _: *mut c_void) -> BOOL {
    let reason = if fdw_reason == SystemServices::DLL_PROCESS_DETACH {
        Reason::DllProcessDetach
    } else if fdw_reason == SystemServices::DLL_PROCESS_ATTACH {
        Reason::DllProcessAttach
    } else {
        return BOOL(1);
    };

    let success = if let Err(e) = main(hinst_dll, reason) {
        println!("{e}");
        false
    } else {
        true
    };

    BOOL(success as i32)
}

fn main(hinst_dll: HINSTANCE, reason: Reason) -> Result<(), Error> {
    if reason == Reason::DllProcessAttach {
        // #[cfg(debug_assertions)]
        unsafe {
            AllocConsole()?;
        }

        unsafe {
            DisableThreadLibraryCalls(hinst_dll)?;
        }

        std::thread::spawn(dll_attach);
    };

    Ok(())
}

fn dll_attach() {
    type ModuleDispatchArray<'a> = &'a [(PCSTR, fn(HMODULE) -> Result<(), Error>)];

    // const SOCKET_NAME: &str = r"\\.\pipe\sylvias_shared_handle.sock";

    const OGL_DLL: PCSTR = s!("opengl32.dll");
    const D3D9_DLL: PCSTR = s!("d3d9.dll");
    const D3D10_DLL: PCSTR = s!("d3d10.dll");
    const D3D11_DLL: PCSTR = s!("d3d11.dll");

    // This is a list of APIs and their hooks, since all APIs need to be attempted to be hooked
    #[allow(unused)]
    const MODULES: ModuleDispatchArray = &[
        (OGL_DLL, dll_attach_rendering_api::<impls::OpenGLHooks>),
        (D3D9_DLL, dll_attach_rendering_api::<impls::DX9Hooks>),
        (D3D10_DLL, dll_attach_rendering_api::<impls::DX10Hooks>),
        (D3D11_DLL, dll_attach_rendering_api::<impls::DX11Hooks>),
    ];

    // let name = SOCKET_NAME.to_ns_name::<GenericNamespaced>().unwrap();

    // let mut try_connect = Stream::connect(name.clone());

    // let mut stream = loop {
    //     match try_connect {
    //         Err(e) if e.kind() == ErrorKind::NotFound => {
    //             try_connect = Stream::connect(name.clone());
    //         }
    //         Err(e) => {
    //             println!("{e}");
    //             try_connect = Stream::connect(name.clone());
    //         }
    //         Ok(stream) => {
    //             break stream;
    //         }
    //     }
    // };

    let shared_buffer = shared_memory::ShmemConf::new()
        .os_id("SylvScreenShare")
        .size(size_of::<NewSharedMemoryHeader>() + size_of::<u32>() * 1920 * 1080)
        .open()
        .unwrap();

    let header = shared_buffer.as_ptr() as *mut NewSharedMemoryHeader;

    unsafe {
        (*header).set_width_and_height(1920, 1080);
    }

    unsafe {
        (*header).set_pid();
    }

    SHARED_CPU_BUFFER.get_or_init(|| crate::SharedMem(shared_buffer));

    let call = unsafe { GetModuleHandleA(OGL_DLL) }
        .map_err(Error::from)
        .and_then(dll_attach_rendering_api::<impls::OpenGLHooks>);
    if let Err(e) = call {
        println!("{e}");
    }

    let call = unsafe { GetModuleHandleA(D3D9_DLL) }
        .map_err(Error::from)
        .and_then(dll_attach_rendering_api::<impls::DX9Hooks>);
    if let Err(e) = call {
        println!("{e}");
    }

    let call = unsafe { GetModuleHandleA(D3D10_DLL) }
        .map_err(Error::from)
        .and_then(dll_attach_rendering_api::<impls::DX10Hooks>);
    if let Err(e) = call {
        println!("{e}");
    }

    let call = unsafe { GetModuleHandleA(D3D11_DLL) }
        .map_err(Error::from)
        .and_then(dll_attach_rendering_api::<impls::DX11Hooks>);
    if let Err(e) = call {
        println!("{e}");
    }
}

fn dll_attach_rendering_api<T>(module: HMODULE) -> Result<(), Error>
where
    T: RenderingAPI,
{
    let rendering_api = T::create(module)?;

    let present_fn = rendering_api.present_fn();

    let detour = unsafe { RawDetour::new(present_fn, T::new_present_fn().to_ptr())? };

    unsafe { detour.enable()? };

    let trampoline = detour.trampoline();

    T::set_trampoline(unsafe { T::PresentFn::from_ptr(trampoline) });
    T::set_detour(detour);

    rendering_api.destroy()?;

    Ok(())
}
