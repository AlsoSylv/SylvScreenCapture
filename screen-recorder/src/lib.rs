// use interprocess::local_socket::traits::Stream as StreamTrait;
// use interprocess::local_socket::{GenericNamespaced, Stream, ToNsName};
use retour::{Function, RawDetour};
use std::ffi::c_void;
// use std::io::{ErrorKind, Read, Write};
use std::sync::OnceLock;
use windows::Win32::Foundation::HMODULE;
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
pub struct SharedMem(pub shmem::Shmem);

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
    const D3D12_DLL: PCSTR = s!("d3d12.dll");
    const VK_DLL: PCSTR = s!("vulkan-1.dll");

    // This is a list of APIs and their hooks, since all APIs need to be attempted to be hooked
    #[allow(unused)]
    const MODULES: ModuleDispatchArray = &[
        (OGL_DLL, dll_attach_rendering_api::<impls::OpenGLHooks>),
        (D3D9_DLL, dll_attach_rendering_api::<impls::DX9Hooks>),
        (D3D10_DLL, dll_attach_rendering_api::<impls::DX10Hooks>),
        (D3D11_DLL, dll_attach_rendering_api::<impls::DX11Hooks>),
        (D3D12_DLL, dll_attach_rendering_api::<impls::DX12Hooks>),
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

    let shared_buffer = shmem::ShmemBuilder::new("SylvScreenShare").open().unwrap();

    let header = shared_buffer.header();
    header.set_pid();

    SHARED_CPU_BUFFER.get_or_init(|| crate::SharedMem(shared_buffer));

    let call = unsafe { GetModuleHandleA(OGL_DLL) }
        .map_err(Error::from)
        .and_then(dll_attach_rendering_api::<impls::OpenGLHooks>);
    if let Err(e) = call {
        println!("{e}");
    }

    // let call = unsafe { GetModuleHandleA(D3D9_DLL) }
    //     .map_err(Error::from)
    //     .and_then(dll_attach_rendering_api::<impls::DX9Hooks>);
    // if let Err(e) = call {
    //     println!("{e}");
    // }

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

    // let call = unsafe { GetModuleHandleA(D3D12_DLL) }
    //     .map_err(Error::from)
    //     .and_then(dll_attach_rendering_api::<impls::DX12Hooks>);
    // if let Err(e) = call {
    //     println!("{e}");
    // }

    let call = unsafe { GetModuleHandleA(VK_DLL) }
        .map_err(Error::from)
        .and_then(dll_attach_rendering_api::<impls::VkHooks>);
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
