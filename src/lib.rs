use retour::RawDetour;
use std::ffi::c_void;
use std::sync::{Once, OnceLock};
use windows::core::{w, PCWSTR};
use windows::Win32::Graphics::Direct3D11::{ID3D11Device1, D3D11_TEXTURE2D_DESC};
use windows::Win32::Graphics::Dxgi::{DXGI_SHARED_RESOURCE_READ, DXGI_SHARED_RESOURCE_WRITE};
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

type PresentFunctionType = unsafe extern "system" fn(*mut c_void, u32, DXGI_PRESENT) -> HRESULT;

static DETOUR: OnceLock<RawDetour> = OnceLock::new();

static SHARED_BUFFER: OnceLock<ID3D11Texture2D> = OnceLock::new();

static TRAMPOLINE: OnceLock<PresentFunctionType> = OnceLock::new();

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
            if let Err(e) = dll_attach() {
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

    TRAMPOLINE.get_or_init(|| tramponline);
    DETOUR.get_or_init(|| detour);

    Ok(())
}

fn new_present_function(this: *mut c_void, sync_internal: u32, flags: DXGI_PRESENT) -> HRESULT {
    const SHARED_WINDOW_NAME: PCWSTR = w!("Sylvia's_Shared_Texture");

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
        let shared_resource_rights = DXGI_SHARED_RESOURCE_READ | DXGI_SHARED_RESOURCE_WRITE;

        let maybe_shared_buffer = unsafe {
            device_1.OpenSharedResourceByName(SHARED_WINDOW_NAME, shared_resource_rights.0)
        };

        match maybe_shared_buffer {
            Ok(shared_buffer) => {
                SHARED_BUFFER.get_or_init(|| shared_buffer);
            }
            Err(e) => {
                println!("Error opening shared texture: {e}")
            }
        }
    }

    let present_function = TRAMPOLINE
        .get()
        .expect("The trampoline was set before this was ever called.");

    unsafe { present_function(this.as_raw(), sync_internal, flags) }
}
