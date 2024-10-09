use core::slice;
use interprocess::local_socket::traits::Stream as StreamTrait;
use interprocess::local_socket::{GenericNamespaced, Stream, ToNsName};
use retour::RawDetour;
use std::ffi::c_void;
use std::io::{ErrorKind, Read, Write};
use std::mem::transmute;
use std::ptr::{addr_of_mut, null, null_mut};
use std::sync::atomic::{AtomicBool, AtomicPtr};
use std::sync::{Once, OnceLock};
use windows::Win32::Foundation::{HANDLE, HMODULE, RECT};
use windows::Win32::Graphics::Direct3D::{D3D_DRIVER_TYPE, D3D_FEATURE_LEVEL};
use windows::Win32::Graphics::Direct3D11::{ID3D11Device1, D3D11_TEXTURE2D_DESC};
use windows::Win32::Graphics::Direct3D9::{
    IDirect3DDevice9, D3DBACKBUFFER_TYPE_MONO, D3DLOCKED_RECT, D3DLOCK_READONLY, D3DPOOL_SYSTEMMEM,
    D3DSURFACE_DESC,
};
use windows::Win32::Graphics::Gdi::{HDC, RGNDATA};
use windows::Win32::System::LibraryLoader::GetProcAddress;
use windows::Win32::System::SystemServices;
use windows::{
    core::{s, Interface, HRESULT, PCSTR},
    Win32::{
        Foundation::{BOOL, HINSTANCE, HWND, LPARAM, LRESULT, WPARAM},
        Graphics::{
            Direct3D::D3D_DRIVER_TYPE_HARDWARE,
            Direct3D11::{
                ID3D11Device, ID3D11Texture2D, D3D11_CREATE_DEVICE_FLAG, D3D11_SDK_VERSION,
            },
            Dxgi::{
                Common::{
                    DXGI_FORMAT_R8G8B8A8_UNORM, DXGI_MODE_DESC, DXGI_MODE_SCALING_UNSPECIFIED,
                    DXGI_MODE_SCANLINE_ORDER_UNSPECIFIED, DXGI_RATIONAL, DXGI_SAMPLE_DESC,
                },
                IDXGISwapChain, DXGI_PRESENT, DXGI_SWAP_CHAIN_DESC,
                DXGI_SWAP_CHAIN_FLAG_ALLOW_MODE_SWITCH, DXGI_SWAP_EFFECT_DISCARD,
                DXGI_USAGE_RENDER_TARGET_OUTPUT,
            },
            Gdi::HBRUSH,
        },
        System::{
            Console::AllocConsole,
            LibraryLoader::{DisableThreadLibraryCalls, GetModuleHandleA},
        },
        UI::WindowsAndMessaging::{
            CreateWindowExA, DefWindowProcA, DestroyWindow, RegisterClassExA, UnregisterClassA,
            CS_HREDRAW, CS_VREDRAW, HCURSOR, HICON, WINDOW_EX_STYLE, WNDCLASSEXA,
            WS_OVERLAPPEDWINDOW,
        },
    },
};

use error::Error;

mod error;
mod impls;

pub trait RenderingAPI: Sized {
    type PresentFn;
    type ResizeFn;

    fn present_fn(&self) -> *const ();

    fn resize_fn(&self) -> *const ();

    fn create(module: HMODULE) -> Result<Self, Error>;

    fn destory(&self) -> Result<(), Error>;
}

#[derive(PartialEq)]
enum Reason {
    DllProcessAttach,
    DllProcessDetach,
}

type Dx9PresentFunctionType = unsafe extern "system" fn(
    *mut c_void,
    *const RECT,
    *const RECT,
    HWND,
    *const RGNDATA,
) -> HRESULT;

type PresentFunctionType = unsafe extern "system" fn(*mut c_void, u32, DXGI_PRESENT) -> HRESULT;

union PresentFunctions {
    dx9: Dx9PresentFunctionType,
}

static DX11_DETOUR: OnceLock<RawDetour> = OnceLock::new();
static DETOUR: OnceLock<RawDetour> = OnceLock::new();

static SHARED_BUFFER: OnceLock<ID3D11Texture2D> = OnceLock::new();

static TRAMPOLINE: OnceLock<PresentFunctions> = OnceLock::new();

static SHARED_HANDLE: AtomicPtr<c_void> = AtomicPtr::new(null_mut());

static OPENGL_SWAP_BUFFERS: OnceLock<<impls::OpenGLHooks as RenderingAPI>::PresentFn> =
    OnceLock::new();

static DXGI_SWAP_BUFFER: OnceLock<PresentFunctionType> = OnceLock::new();

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
        #[cfg(debug_assertions)]
        unsafe {
            AllocConsole()?;
        }

        unsafe {
            DisableThreadLibraryCalls(hinst_dll)?;
        }

        std::thread::spawn(|| {
            const OGL_DLL: PCSTR = s!("opengl32.dll");
            const D3D9_DLL: PCSTR = s!("d3d9.dll");
            const D3D11_DLL: PCSTR = s!("d3d11.dll");

            let call = unsafe { GetModuleHandleA(OGL_DLL) }
                .map_err(Error::from)
                .and_then(dll_attach_ogl);
            if let Err(e) = call {
                println!("{e}");
            }

            let call = unsafe { GetModuleHandleA(D3D9_DLL) }
                .map_err(Error::from)
                .and_then(dll_attach_dx9);
            if let Err(e) = call {
                println!("{e}");
            }

            let module = unsafe { GetModuleHandleA(D3D11_DLL) };

            println!("{module:?}");

            if let Ok(module) = module {
                if let Err(e) = dll_attach(module) {
                    println!("{e:?}")
                }
            }

            // let call = unsafe { GetModuleHandleA(D3D11_DLL) }
            //     .map_err(Error::from)
            //     .and(dll_attach());
            // if let Err(e) = call {
            //     println!("{e}");
            // }
        });
    };

    Ok(())
}

// Workaround
unsafe extern "system" fn def_window_pro_a(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    DefWindowProcA(hwnd, msg, wparam, lparam)
}

fn dll_attach_ogl(module: HMODULE) -> Result<(), Error> {
    let gl = impls::OpenGLHooks::create(module)?;
    let func = gl.present_fn();
    let detour = unsafe { RawDetour::new(func, new_wgl_swap_buffers as _)? };

    unsafe { detour.enable()? };

    let tramponline: <impls::OpenGLHooks as RenderingAPI>::PresentFn =
        unsafe { std::mem::transmute(detour.trampoline()) };

    OPENGL_SWAP_BUFFERS.get_or_init(|| tramponline);
    DETOUR.get_or_init(|| detour);

    let name = r"\\.\pipe\sylvias_shared_handle.sock"
        .to_ns_name::<GenericNamespaced>()
        .unwrap();

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

    gl.destory()?;

    Ok(())
}

fn new_wgl_swap_buffers(un_named_1: HDC) -> BOOL {
    use glad_gl::gl;

    WAS_OPENGL_CALL.store(true, std::sync::atomic::Ordering::SeqCst);

    let handle = HANDLE(SHARED_HANDLE.load(std::sync::atomic::Ordering::Relaxed));

    if !handle.is_invalid() {
        let (mut memory_object, mut texture) = (0, 0);

        unsafe { gl::CreateMemoryObjectsEXT(1, addr_of_mut!(memory_object)) };
        unsafe {
            gl::ImportMemoryWin32HandleEXT(
                memory_object,
                0,
                gl::HANDLE_TYPE_D3D11_IMAGE_EXT,
                handle.0,
            )
        };

        unsafe { gl::GenTextures(1, addr_of_mut!(texture)) };
        unsafe { gl::BindTexture(gl::TEXTURE_2D, texture) };

        unsafe {
            gl::TexStorageMem2DEXT(gl::TEXTURE_2D, 1, gl::RGBA8, 1920, 1080, memory_object, 0)
        };
        unsafe { gl::CopyTexSubImage2D(gl::TEXTURE_2D, 0, 0, 0, 0, 0, 1920, 1080) };
        unsafe { gl::DeleteTextures(1, &texture) };
        unsafe { gl::DeleteMemoryObjectsEXT(1, &memory_object) };
    }

    let present_function = OPENGL_SWAP_BUFFERS
        .get()
        .expect("The trampoline was set before this was ever called.");

    unsafe { (present_function)(un_named_1) }
}

#[allow(unused)]
fn dll_attach_dx9(module: HMODULE) -> Result<(), Error> {
    let d3d9 = impls::DX9Hooks::create(module)?;

    let present = d3d9.present_fn();

    let detour = unsafe { RawDetour::new(present, new_dx9_present_function as _)? };

    unsafe { detour.enable()? };

    let tramponline: Dx9PresentFunctionType = unsafe { std::mem::transmute(detour.trampoline()) };

    TRAMPOLINE.get_or_init(|| PresentFunctions { dx9: tramponline });
    DETOUR.get_or_init(|| detour);

    d3d9.destory()?;

    let name = r"\\.\pipe\sylvias_shared_handle.sock"
        .to_ns_name::<GenericNamespaced>()
        .unwrap();

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

    let shared_buffer = shared_memory::ShmemConf::new()
        .os_id("SylvScreenShare")
        .size(size_of::<u32>() * 2 + size_of::<u32>() * 1920 * 1080)
        .open()
        .unwrap();

    SHARED_CPU_BUFFER.get_or_init(|| SharedMem(shared_buffer));

    Ok(())
}

#[repr(transparent)]
struct SharedMem(shared_memory::Shmem);

unsafe impl Send for SharedMem {}
unsafe impl Sync for SharedMem {}

static SHARED_CPU_BUFFER: OnceLock<SharedMem> = OnceLock::new();

pub static WAS_OPENGL_CALL: AtomicBool = AtomicBool::new(false);

fn new_dx9_present_function(
    this: *mut c_void,
    src_rect: *const RECT,
    dst_rect: *const RECT,
    window: HWND,
    rgn: *const RGNDATA,
) -> HRESULT {
    WAS_OPENGL_CALL.store(true, std::sync::atomic::Ordering::SeqCst);

    let this = unsafe { IDirect3DDevice9::from_raw(this) };

    if let Some(buffer) = SHARED_CPU_BUFFER.get() {
        let buffer_ptr = buffer.0.as_ptr();

        let back_buffer = unsafe { this.GetBackBuffer(0, 0, D3DBACKBUFFER_TYPE_MONO).unwrap() };

        let mut desc = D3DSURFACE_DESC::default();

        unsafe { back_buffer.GetDesc(&mut desc).unwrap() };

        let width = desc.Width;
        let height = desc.Height;

        let mut out_surf = None;

        unsafe {
            this.CreateOffscreenPlainSurface(
                width,
                height,
                desc.Format,
                D3DPOOL_SYSTEMMEM,
                &mut out_surf,
                null_mut(),
            )
            .unwrap()
        };

        let out_surf = out_surf.unwrap();

        unsafe { this.GetRenderTargetData(&back_buffer, &out_surf).unwrap() };

        let mut locked_rect = D3DLOCKED_RECT::default();
        let read_only = D3DLOCK_READONLY as u32;

        unsafe {
            out_surf
                .LockRect(&mut locked_rect, null(), read_only)
                .unwrap();
        }

        let step_by = locked_rect.Pitch as usize * height as usize;

        let slice = unsafe {
            slice::from_raw_parts_mut(locked_rect.pBits.cast::<u8>(), step_by * width as usize * 4)
        };

        let width_bytes = width.to_le_bytes();
        let height_bytes = height.to_le_bytes();
        unsafe {
            buffer_ptr.copy_from(width_bytes.as_ptr(), 4);
        }
        unsafe {
            buffer_ptr.add(4).copy_from(height_bytes.as_ptr(), 4);
        }

        let buffer_ptr = unsafe { buffer_ptr.add(size_of::<u32>() * 2) };
        for y in 0..desc.Height as usize {
            let location = locked_rect.Pitch as usize * y;
            let slice = &mut slice[location..(location + width as usize * 4)];

            // TODO: Move this to the parent process?
            // Simple way to encode the BGRA chunk to RGBA
            slice.chunks_mut(4).for_each(|slice| {
                assert!(slice.len() == 4);
                slice.swap(0, 2);
                slice[3] = 255;
            });

            let buffer_ptr = unsafe { buffer_ptr.add(4 * y * width as usize) };
            unsafe { buffer_ptr.copy_from(slice.as_ptr(), slice.len()) };
        }

        unsafe { out_surf.UnlockRect().unwrap() }
    }

    let present_fn_union = TRAMPOLINE
        .get()
        .expect("The trampoline was set before this was ever called.");

    // SAFETY: This is set when the app is running in DX9
    // so because we're already here we know that the present function has to be DX9
    let present_function = unsafe { present_fn_union.dx9 };

    // SAFETY: This is the original present function to be called
    unsafe { (present_function)(this.as_raw(), src_rect, dst_rect, window, rgn) }
}

#[allow(unused)]
fn dll_attach(module: HMODULE) -> Result<(), Error> {
    const WINDOW_CLASS_NAME: PCSTR = s!("dummy_window_for_swap_chain");

    let window_class = WNDCLASSEXA {
        cbSize: size_of::<WNDCLASSEXA>() as u32,
        style: CS_HREDRAW | CS_VREDRAW,
        lpfnWndProc: Some(def_window_pro_a),
        hInstance: unsafe { GetModuleHandleA(None).unwrap().into() },
        lpszClassName: WINDOW_CLASS_NAME,
        cbClsExtra: 0,
        cbWndExtra: 0,
        hIcon: HICON::default(),
        hCursor: HCURSOR::default(),
        hIconSm: HICON::default(),
        hbrBackground: HBRUSH::default(),
        lpszMenuName: PCSTR::null(),
    };

    let registered_window_class = unsafe { RegisterClassExA(&window_class) };

    if registered_window_class == 0 {
        Err(windows::core::Error::from_win32())?
    }

    let window = unsafe {
        CreateWindowExA(
            WINDOW_EX_STYLE::default(),
            WINDOW_CLASS_NAME,
            WINDOW_CLASS_NAME,
            WS_OVERLAPPEDWINDOW,
            0,
            0,
            100,
            100,
            None,
            None,
            window_class.hInstance,
            None,
        )?
    };

    let swap_chain_desc = DXGI_SWAP_CHAIN_DESC {
        BufferDesc: DXGI_MODE_DESC {
            Width: 100,
            Height: 100,
            RefreshRate: DXGI_RATIONAL {
                Numerator: 60,
                Denominator: 1,
            },
            Format: DXGI_FORMAT_R8G8B8A8_UNORM,
            ScanlineOrdering: DXGI_MODE_SCANLINE_ORDER_UNSPECIFIED,
            Scaling: DXGI_MODE_SCALING_UNSPECIFIED,
        },
        SampleDesc: DXGI_SAMPLE_DESC {
            Count: 1,
            Quality: 0,
        },
        BufferUsage: DXGI_USAGE_RENDER_TARGET_OUTPUT,
        BufferCount: 1,
        OutputWindow: window,
        Windowed: BOOL(true as i32),
        SwapEffect: DXGI_SWAP_EFFECT_DISCARD,
        Flags: DXGI_SWAP_CHAIN_FLAG_ALLOW_MODE_SWITCH.0 as u32,
    };

    let mut swap_chain: *mut IDXGISwapChain = null_mut();

    type D3D11Create = unsafe extern "system" fn(
        padapter: *mut c_void,
        drivertype: D3D_DRIVER_TYPE,
        software: HMODULE,
        flags: D3D11_CREATE_DEVICE_FLAG,
        pfeaturelevels: *const D3D_FEATURE_LEVEL,
        featurelevels: u32,
        sdkversion: u32,
        pswapchaindesc: *const DXGI_SWAP_CHAIN_DESC,
        ppswapchain: *mut *mut c_void,
        ppdevice: *mut *mut c_void,
        pfeaturelevel: *mut D3D_FEATURE_LEVEL,
        ppimmediatecontext: *mut *mut c_void,
    ) -> HRESULT;

    let create_device =
        unsafe { GetProcAddress(module, s!("D3D11CreateDeviceAndSwapChain")).unwrap() };

    #[allow(non_snake_case)]
    let D3D11CreateDeviceAndSwapChain: D3D11Create = unsafe { transmute(create_device) };

    unsafe {
        let result = D3D11CreateDeviceAndSwapChain(
            null_mut(),
            D3D_DRIVER_TYPE_HARDWARE,
            HMODULE(null_mut()),
            D3D11_CREATE_DEVICE_FLAG(0),
            null_mut(),
            0,
            D3D11_SDK_VERSION,
            &swap_chain_desc,
            &mut addr_of_mut!(swap_chain).cast(),
            null_mut(),
            null_mut(),
            null_mut(),
        );

        if result.is_err() {
            return Err(windows::core::Error::from_win32().into());
        }
    };

    assert!(!swap_chain.is_null());

    let swap_chain = Some(unsafe { (*swap_chain).clone() });

    if let Some(swap_chain) = swap_chain {
        let present_function: PresentFunctionType = swap_chain.vtable().Present;
        let detour = unsafe { RawDetour::new(present_function as _, new_present_function as _)? };

        unsafe { detour.enable()? };

        let tramponline: PresentFunctionType = unsafe { std::mem::transmute(detour.trampoline()) };

        DXGI_SWAP_BUFFER.get_or_init(|| tramponline);
        DX11_DETOUR.get_or_init(|| detour);

        let name = r"\\.\pipe\sylvias_shared_handle.sock"
            .to_ns_name::<GenericNamespaced>()
            .unwrap();

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
    }

    unsafe {
        DestroyWindow(window)?;
        UnregisterClassA(WINDOW_CLASS_NAME, window_class.hInstance)?;
    }

    Ok(())
}

fn new_present_function(this: *mut c_void, sync_internal: u32, flags: DXGI_PRESENT) -> HRESULT {
    let present_function = DXGI_SWAP_BUFFER
        .get()
        .expect("The trampoline was set before this was ever called.");

    println!("h");

    if WAS_OPENGL_CALL.load(std::sync::atomic::Ordering::SeqCst) {
        WAS_OPENGL_CALL.store(false, std::sync::atomic::Ordering::SeqCst);

        return unsafe { present_function(this, sync_internal, flags) };
    }

    let this = unsafe { IDXGISwapChain::from_raw(this) };

    let device: ID3D11Device = unsafe { this.GetDevice() }.unwrap();
    let device_1: ID3D11Device1 = device.cast().unwrap();

    if let Some(shared_buffer) = SHARED_BUFFER.get() {
        let context = unsafe { device.GetImmediateContext() }.unwrap();
        let back_buffer: ID3D11Texture2D = unsafe { this.GetBuffer(0) }.unwrap();

        #[cfg(debug_assertions)]
        {
            static DESCRIPTION: Once = Once::new();

            DESCRIPTION.call_once(|| {
                let mut desc = D3D11_TEXTURE2D_DESC::default();

                unsafe { back_buffer.GetDesc(&mut desc) };

                println!("{:?}", desc);
            });
        }

        unsafe { context.CopyResource(shared_buffer, &back_buffer) };
    } else {
        let handle = SHARED_HANDLE.load(std::sync::atomic::Ordering::Relaxed);

        if !handle.is_null() {
            let maybe_shared_buffer = unsafe { device_1.OpenSharedResource1(HANDLE(handle)) };

            match maybe_shared_buffer {
                Ok(shared_buffer) => {
                    SHARED_BUFFER.get_or_init(|| shared_buffer);
                }
                Err(e) => {
                    println!("Error opening shared texture: {e}")
                }
            }
        }
    }

    unsafe { (present_function)(this.as_raw(), sync_internal, flags) }
}
