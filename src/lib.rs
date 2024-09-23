use core::slice;
use interprocess::local_socket::traits::Stream as StreamTrait;
use interprocess::local_socket::{GenericNamespaced, Stream, ToNsName};
use retour::RawDetour;
use std::ffi::c_void;
use std::fs::File;
use std::io::{BufWriter, ErrorKind, Read, Write};
use std::ops::Mul;
use std::ptr::{null, null_mut};
use std::sync::atomic::AtomicPtr;
use std::sync::{Once, OnceLock};
use windows::Win32::Foundation::{HANDLE, RECT};
use windows::Win32::Graphics::Direct3D11::{ID3D11Device1, D3D11_TEXTURE2D_DESC};
use windows::Win32::Graphics::Direct3D9::{
    D3D9b_SDK_VERSION, Direct3DCreate9, Direct3DCreate9Ex, IDirect3DDevice9, IDirect3DDevice9Ex,
    IDirect3DSurface9, IDirect3DTexture9, D3DBACKBUFFER_TYPE_MONO,
    D3DCREATE_HARDWARE_VERTEXPROCESSING, D3DDEVTYPE_HAL, D3DDISPLAYMODEEX, D3DFMT_A8R8G8B8,
    D3DFMT_UNKNOWN, D3DFMT_X8R8G8B8, D3DLOCKED_RECT, D3DLOCK_READONLY, D3DMULTISAMPLE_NONE,
    D3DPOOL_DEFAULT, D3DPOOL_SYSTEMMEM, D3DPRESENTFLAG_DEVICECLIP, D3DPRESENT_PARAMETERS,
    D3DSURFACE_DESC, D3DSWAPEFFECT_COPY, D3DTEXF_NONE, D3DUSAGE_DYNAMIC, D3DUSAGE_RENDERTARGET,
};
use windows::Win32::Graphics::Gdi::RGNDATA;
use windows::Win32::System::SystemServices;
use windows::{
    core::{s, Interface, HRESULT, PCSTR},
    Win32::{
        Foundation::{BOOL, HINSTANCE, HWND, LPARAM, LRESULT, WPARAM},
        Graphics::{
            Direct3D::D3D_DRIVER_TYPE_HARDWARE,
            Direct3D11::{
                D3D11CreateDeviceAndSwapChain, ID3D11Device, ID3D11Texture2D,
                D3D11_CREATE_DEVICE_FLAG, D3D11_SDK_VERSION,
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

mod error {
    #[derive(Debug)]
    pub enum Error {
        Win32(windows::core::Error),
        Detour(retour::Error),
    }

    impl std::fmt::Display for Error {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            match self {
                Error::Win32(e) => e.fmt(f),
                Error::Detour(e) => e.fmt(f),
            }
        }
    }

    impl std::error::Error for Error {}

    impl From<retour::Error> for Error {
        fn from(value: retour::Error) -> Self {
            Error::Detour(value)
        }
    }

    impl From<windows::core::Error> for Error {
        fn from(value: windows::core::Error) -> Self {
            Error::Win32(value)
        }
    }
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
    dx11: PresentFunctionType,
}

static DETOUR: OnceLock<RawDetour> = OnceLock::new();

static SHARED_BUFFER: OnceLock<ID3D11Texture2D> = OnceLock::new();

static DX9_SHARED_BUFFER: AtomicPtr<c_void> = AtomicPtr::new(null_mut());

static TRAMPOLINE: OnceLock<PresentFunctions> = OnceLock::new();

static SHARED_HANDLE: AtomicPtr<c_void> = AtomicPtr::new(null_mut());

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
        unsafe {
            AllocConsole()?;
        }

        unsafe {
            DisableThreadLibraryCalls(hinst_dll)?;
        }

        std::thread::spawn(|| {
            if let Err(e) = dll_attach_dx9() {
                println!("{e}");
            }
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

fn dll_attach_dx9() -> Result<(), Error> {
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

    let d3d9 = unsafe { Direct3DCreate9Ex(D3D9b_SDK_VERSION)? };

    let mut present_params = D3DPRESENT_PARAMETERS {
        BackBufferWidth: 100,
        BackBufferHeight: 100,
        BackBufferFormat: D3DFMT_UNKNOWN,
        BackBufferCount: 1,
        MultiSampleType: D3DMULTISAMPLE_NONE,
        MultiSampleQuality: 0,
        SwapEffect: D3DSWAPEFFECT_COPY,
        hDeviceWindow: window,
        Windowed: true.into(),
        EnableAutoDepthStencil: false.into(),
        AutoDepthStencilFormat: D3DFMT_UNKNOWN,
        Flags: D3DPRESENTFLAG_DEVICECLIP,
        FullScreen_RefreshRateInHz: 0,
        PresentationInterval: 1,
    };

    let mut device = None;

    unsafe {
        d3d9.CreateDevice(
            0,
            D3DDEVTYPE_HAL,
            window,
            D3DCREATE_HARDWARE_VERTEXPROCESSING as u32,
            &mut present_params,
            &mut device,
        )?
    };

    let deviceex = device.unwrap();

    let present = deviceex.vtable().Present;

    unsafe {
        DestroyWindow(window)?;
        UnregisterClassA(WINDOW_CLASS_NAME, window_class.hInstance)?;
    }

    let detour = unsafe { RawDetour::new(present as _, new_dx9_present_function as _)? };

    unsafe { detour.enable()? };

    let tramponline: Dx9PresentFunctionType = unsafe { std::mem::transmute(detour.trampoline()) };

    TRAMPOLINE.get_or_init(|| PresentFunctions { dx9: tramponline });
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

    stream.write(&std::process::id().to_le_bytes()).unwrap();
    let mut handle = [0; 8];
    stream.read_exact(&mut handle).unwrap();
    let ptr = isize::from_le_bytes(handle);
    SHARED_HANDLE.store(ptr as _, std::sync::atomic::Ordering::Relaxed);

    Ok(())
}

fn new_dx9_present_function(
    this: *mut c_void,
    src_rect: *const RECT,
    dst_rect: *const RECT,
    window: HWND,
    rgn: *const RGNDATA,
) -> HRESULT {
    let this = unsafe { IDirect3DDevice9::from_raw(this) };

    let rec = unsafe { &*src_rect };

    let width = (rec.right - rec.left) as u32;
    let height = (rec.bottom - rec.top) as u32;

    let render_target = unsafe { this.GetRenderTarget(0).unwrap() };

    let mut desc = D3DSURFACE_DESC::default();
    unsafe { render_target.GetDesc(&mut desc).unwrap() };

    let mut out_surf = None;
    unsafe {
        this.CreateOffscreenPlainSurface(
            desc.Width,
            desc.Height,
            desc.Format,
            D3DPOOL_SYSTEMMEM,
            &mut out_surf,
            null_mut(),
        )
        .unwrap()
    };

    let out_surf = out_surf.unwrap();

    unsafe { this.GetRenderTargetData(&render_target, &out_surf).unwrap() };

    static ONCE: Once = Once::new();

    ONCE.call_once(|| {
        std::thread::sleep_ms(1000 * 5);

        let mut locked_rect = D3DLOCKED_RECT::default();

        unsafe {
            out_surf
                .LockRect(&mut locked_rect, null(), D3DLOCK_READONLY as u32)
                .unwrap();
        }

        println!("{:?}", desc);
        println!("{width} vs 1920 {height} vs 1080");

        let slice = unsafe {
            slice::from_raw_parts(
                locked_rect.pBits as *const u8,
                (width).mul(height).mul(4) as usize,
            )
        };

        let path = File::create(std::env::home_dir().unwrap().join("screenshot.png")).unwrap();
        let writer = BufWriter::new(path);

        let mut encoder = png::Encoder::new(writer, width, height);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);

        let mut writer = encoder.write_header().unwrap();

        writer.write_image_data(slice).unwrap();

        unsafe { out_surf.UnlockRect().unwrap() }
    });

    let present_function = TRAMPOLINE
        .get()
        .expect("The trampoline was set before this was ever called.");

    unsafe { (present_function.dx9)(this.as_raw(), src_rect, dst_rect, window, rgn) }
}

fn dll_attach() -> Result<(), Error> {
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

    let mut swap_chain: Option<IDXGISwapChain> = None;

    unsafe {
        D3D11CreateDeviceAndSwapChain(
            None,
            D3D_DRIVER_TYPE_HARDWARE,
            None,
            D3D11_CREATE_DEVICE_FLAG(0),
            None,
            D3D11_SDK_VERSION,
            Some(&swap_chain_desc),
            Some(&mut swap_chain),
            None,
            None,
            None,
        )?;
    };

    let swap_chain = swap_chain.unwrap();

    let present_function: PresentFunctionType = swap_chain.vtable().Present;

    unsafe {
        DestroyWindow(window)?;
        UnregisterClassA(WINDOW_CLASS_NAME, window_class.hInstance)?;
    }

    let detour = unsafe { RawDetour::new(present_function as _, new_present_function as _)? };

    unsafe { detour.enable()? };

    let tramponline: PresentFunctionType = unsafe { std::mem::transmute(detour.trampoline()) };

    TRAMPOLINE.get_or_init(|| PresentFunctions { dx11: tramponline });
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

    stream.write(&std::process::id().to_le_bytes()).unwrap();
    let mut handle = [0; 8];
    stream.read_exact(&mut handle).unwrap();
    let ptr = isize::from_le_bytes(handle);
    SHARED_HANDLE.store(ptr as _, std::sync::atomic::Ordering::Relaxed);

    Ok(())
}

fn new_present_function(this: *mut c_void, sync_internal: u32, flags: DXGI_PRESENT) -> HRESULT {
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

    let present_function = TRAMPOLINE
        .get()
        .expect("The trampoline was set before this was ever called.");

    unsafe { (present_function.dx11)(this.as_raw(), sync_internal, flags) }
}
