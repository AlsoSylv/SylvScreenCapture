// use interprocess::local_socket::traits::Stream as StreamTrait;
// use interprocess::local_socket::{GenericNamespaced, Stream, ToNsName};
use retour::{Function, RawDetour};
use std::ffi::c_void;
use windows::core::BOOL;
// use std::io::{ErrorKind, Read, Write};
use std::sync::{LazyLock, RwLock};
use windows::Win32::Foundation::HMODULE;
use windows::Win32::System::SystemServices;
use windows::{
    Win32::{
        Foundation::HINSTANCE,
        System::{
            Console::AllocConsole,
            LibraryLoader::{DisableThreadLibraryCalls, GetModuleHandleA},
        },
    },
    core::{PCSTR, s},
};

use error::Error;

mod error;
mod impls;

/*
TODO: there needs to be support for more than one present fn
This could either take the form of passing an array via const generic or assoc consts
Or this could use a slice, though I don't think that would work given the fact that a detour
And trampoline function are required
*/
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

/// This requires that the shared memory be created BEFORE the DLL is injected, but this is fine
/// This is wrapped in a RwLock, not for safety (every operation is atomic), but so that it can be dropped
/// When the game exits
pub static SHARED_CPU_BUFFER: LazyLock<RwLock<shmem::Shmem<shared_defs::SharedMemoryHeader>>> =
    LazyLock::new(|| {
        use std::io::Write;

        const START: &str = "SylvScreenShare";
        // This is length + u32::MAX.to_string().len() + null
        let mut name = [0; START.len() + 11];
        write!(name.as_mut_slice(), "{START}{}", std::process::id()).unwrap();
        let name = std::ffi::CStr::from_bytes_until_nul(&name).unwrap();
        let shared_buffer = shmem::Shmem::<shared_defs::SharedMemoryHeader>::open(name);
        shared_buffer.set_loaded(true);
        shared_buffer.set_pid();
        RwLock::new(shared_buffer)
    });

// Export this main as DllMain
#[unsafe(export_name = "DllMain")]
pub extern "system" fn dll_main(hinst_dll: HINSTANCE, fdw_reason: u32, _: *mut c_void) -> BOOL {
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
        #[cfg(debug_assertions)]
        unsafe {
            if let Err(e) = AllocConsole() {
                eprintln!("{e:?}")
            }
        }

        unsafe {
            DisableThreadLibraryCalls(hinst_dll.into())?;
        }

        std::thread::spawn(dll_attach);
    } else {
        let mut lock = SHARED_CPU_BUFFER.write().unwrap();
        lock.set_api(shared_defs::RenderingAPI::None);
        unsafe { lock.dec_program_count() };
    };

    Ok(())
}

fn dll_attach() {
    type ModuleDispatchArray<'a> = &'a [(PCSTR, fn(HMODULE) -> Result<(), Error>)];

    /* TODO: DX10, 11, and 12 share the same characteristics and all use DXGI, and can be checked for at run time.
       Individual hooks should be replaced with a single DXGIHook that does this.
    */
    const OGL_DLL: PCSTR = s!("opengl32.dll");
    // This codepath has been (tempirarily) disabled
    const D3D9_DLL: PCSTR = s!("d3d9.dll");
    const D3D10_DLL: PCSTR = s!("d3d10.dll");
    const D3D11_DLL: PCSTR = s!("d3d11.dll");
    const D3D12_DLL: PCSTR = s!("d3d12.dll");
    const VK_DLL: PCSTR = s!("vulkan-1.dll");

    // This is a list of APIs and their hooks, since all APIs need to be attempted to be hooked
    const MODULES: ModuleDispatchArray<'static> = &[
        (OGL_DLL, dll_attach_rendering_api::<impls::OpenGLHooks>),
        // (D3D9_DLL, dll_attach_rendering_api::<impls::DX9Hooks>),
        (D3D10_DLL, dll_attach_rendering_api::<impls::DX10Hooks>),
        (D3D11_DLL, dll_attach_rendering_api::<impls::DX11Hooks>),
        (D3D12_DLL, dll_attach_rendering_api::<impls::DX12Hooks>),
        (VK_DLL, dll_attach_rendering_api::<impls::VkHooks>),
    ];

    for (dll, hook) in MODULES {
        print!("Trying to hook into: {:?}", unsafe { dll.to_string() });
        let call = unsafe { GetModuleHandleA(*dll) }
            .map_err(Error::from)
            .and_then(*hook);
        if let Err(e) = call {
            print!("Failed to hook: {:?}, code: {e}", unsafe { dll.to_string() });
        }
        println!()
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
