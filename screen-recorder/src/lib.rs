use interprocess::local_socket::traits::Stream as StreamTrait;
use interprocess::local_socket::{GenericNamespaced, Stream, ToNsName};
use retour::{Function, RawDetour};
use std::ffi::c_void;
use std::io::{ErrorKind, Read, Write};
use std::ptr::null_mut;
use std::sync::atomic::{AtomicBool, AtomicPtr};
use std::sync::OnceLock;
use windows::Win32::Foundation::HMODULE;
use windows::Win32::System::SystemServices;
use windows::{
    core::{s, PCSTR},
    Win32::{
        Foundation::{BOOL, HINSTANCE},
        Graphics::Direct3D11::ID3D11Texture2D,
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
    type PresentFn: retour::Function;
    type ResizeFn;

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

static SHARED_BUFFER: OnceLock<ID3D11Texture2D> = OnceLock::new();

#[repr(transparent)]
struct SharedMem(shared_memory::Shmem);

unsafe impl Send for SharedMem {}
unsafe impl Sync for SharedMem {}

static SHARED_CPU_BUFFER: OnceLock<SharedMem> = OnceLock::new();

pub static WAS_OPENGL_CALL: AtomicBool = AtomicBool::new(false);

pub static SHARED_HANDLE: AtomicPtr<c_void> = AtomicPtr::new(null_mut());

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

        std::thread::spawn(|| {
            const SOCKET_NAME: &str = r"\\.\pipe\sylvias_shared_handle.sock";

            const OGL_DLL: PCSTR = s!("opengl32.dll");
            const D3D9_DLL: PCSTR = s!("d3d9.dll");
            const D3D11_DLL: PCSTR = s!("d3d11.dll");

            let name = SOCKET_NAME.to_ns_name::<GenericNamespaced>().unwrap();

            let mut try_connect = Stream::connect(name.clone());

            let mut stream = loop {
                match try_connect {
                    Err(e) if e.kind() == ErrorKind::NotFound => {
                        try_connect = Stream::connect(name.clone());
                    }
                    Err(e) => {
                        println!("{e}");
                        try_connect = Stream::connect(name.clone());
                    }
                    Ok(stream) => {
                        break stream;
                    }
                }
            };

            stream.write_all(&std::process::id().to_le_bytes()).unwrap();
            let mut handle = [0; 8];
            stream.read_exact(&mut handle).unwrap();
            let ptr = isize::from_le_bytes(handle);
            SHARED_HANDLE.store(ptr as _, std::sync::atomic::Ordering::Relaxed);

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

            let call = unsafe { GetModuleHandleA(D3D11_DLL) }
                .map_err(Error::from)
                .and_then(dll_attach_rendering_api::<impls::DX11Hooks>);
            if let Err(e) = call {
                println!("{e}");
            }
        });
    };

    Ok(())
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
