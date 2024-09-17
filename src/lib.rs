use retour::RawDetour;
use std::ffi::c_void;
use std::mem::ManuallyDrop;
use std::ptr::null_mut;
use std::sync::{Once, OnceLock};
use windows::core::w;
use windows::Win32::Graphics::Direct3D11::{ID3D11Device1, D3D11_TEXTURE2D_DESC};
use windows::Win32::Graphics::Dxgi::{DXGI_SHARED_RESOURCE_READ, DXGI_SHARED_RESOURCE_WRITE};
use windows::{
    core::{s, Interface, HRESULT, PCSTR},
    Win32::{
        Foundation::{BOOL, HINSTANCE, HMODULE, HWND, LPARAM, LRESULT, WPARAM},
        Graphics::{
            Direct3D::{
                D3D_DRIVER_TYPE_HARDWARE, D3D_FEATURE_LEVEL, D3D_FEATURE_LEVEL_10_1,
                D3D_FEATURE_LEVEL_11_0,
            },
            Direct3D11::{
                D3D11CreateDeviceAndSwapChain, ID3D11Device, ID3D11DeviceContext, ID3D11Texture2D,
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
            LibraryLoader::{DisableThreadLibraryCalls, GetModuleHandleA, GetProcAddress},
            SystemServices::DLL_PROCESS_ATTACH,
        },
        UI::WindowsAndMessaging::{
            CreateWindowExA, DefWindowProcA, DestroyWindow, RegisterClassExA, UnregisterClassA,
            CS_HREDRAW, CS_VREDRAW, HCURSOR, HICON, WINDOW_EX_STYLE, WNDCLASSEXA,
            WS_OVERLAPPEDWINDOW,
        },
    },
};

// Export this main as DllMain
#[export_name = "DllMain"]
pub extern "stdcall" fn main(
    hinst_dll: HINSTANCE,
    fdw_reason: u32,
    _lpv_reserved: *mut c_void,
) -> BOOL {
    unsafe {
        DisableThreadLibraryCalls(hinst_dll).unwrap();
    }

    if fdw_reason == DLL_PROCESS_ATTACH {
        #[cfg(debug_assertions)]
        unsafe {
            AllocConsole().unwrap();
        }

        std::thread::spawn(|| {
            dll_attach(null_mut());
        });
    }

    true.into()
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

fn dll_attach(_base: *mut c_void) -> u32 {
    const WINDOW_CLASS_NAME: PCSTR = s!("dummy_window_for_swap_chain");
    const DX_MODULE_NAME: PCSTR = s!("d3d11.dll");
    const SWAP_CHAIN_FUNCTION_NAME: PCSTR = s!("D3D11CreateDeviceAndSwapChain");

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
        return 1;
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
        )
        .unwrap()
    };

    if window.0.is_null() {
        return 1;
    }

    let lib_d3d11: HMODULE = unsafe { GetModuleHandleA(DX_MODULE_NAME).unwrap() };
    if lib_d3d11.0.is_null() {
        return 1;
    }

    let d3d11_create_device_and_swap_chain =
        unsafe { GetProcAddress(lib_d3d11, SWAP_CHAIN_FUNCTION_NAME) };
    if d3d11_create_device_and_swap_chain.is_none() {
        return 1;
    }

    let refresh_rate = DXGI_RATIONAL {
        Numerator: 60,
        Denominator: 1,
    };
    let buffer_desc = DXGI_MODE_DESC {
        Width: 100,
        Height: 100,
        RefreshRate: refresh_rate,
        Format: DXGI_FORMAT_R8G8B8A8_UNORM,
        ScanlineOrdering: DXGI_MODE_SCANLINE_ORDER_UNSPECIFIED,
        Scaling: DXGI_MODE_SCALING_UNSPECIFIED,
    };
    let sample_desc = DXGI_SAMPLE_DESC {
        Count: 1,
        Quality: 0,
    };
    let swap_chain_desc = DXGI_SWAP_CHAIN_DESC {
        BufferDesc: buffer_desc,
        SampleDesc: sample_desc,
        BufferUsage: DXGI_USAGE_RENDER_TARGET_OUTPUT,
        BufferCount: 1,
        OutputWindow: window,
        Windowed: true.into(),
        SwapEffect: DXGI_SWAP_EFFECT_DISCARD,
        Flags: DXGI_SWAP_CHAIN_FLAG_ALLOW_MODE_SWITCH.0 as u32,
    };

    let mut swap_chain: Option<IDXGISwapChain> = None;
    let mut device: Option<ID3D11Device> = None;
    let mut context: Option<ID3D11DeviceContext> = None;
    let mut d3d_feature_level: D3D_FEATURE_LEVEL = D3D_FEATURE_LEVEL::default();

    unsafe {
        D3D11CreateDeviceAndSwapChain(
            None,
            D3D_DRIVER_TYPE_HARDWARE,
            None,
            D3D11_CREATE_DEVICE_FLAG::default(),
            Some(&[D3D_FEATURE_LEVEL_10_1, D3D_FEATURE_LEVEL_11_0]),
            D3D11_SDK_VERSION,
            Some(&swap_chain_desc),
            Some(&mut swap_chain),
            Some(&mut device),
            Some(&mut d3d_feature_level),
            Some(&mut context),
        )
        .unwrap();
    };

    let Some(swap_chain) = swap_chain else {
        return 1;
    };

    let present_function: unsafe extern "system" fn(*mut c_void, u32, DXGI_PRESENT) -> HRESULT =
        swap_chain.vtable().Present;

    unsafe {
        DestroyWindow(window).unwrap();
        UnregisterClassA(WINDOW_CLASS_NAME, window_class.hInstance).unwrap();
    }

    let new_detour = unsafe { RawDetour::new(present_function as _, new_present_function as _) };

    match new_detour {
        Ok(detour) => {
            let detour = ManuallyDrop::new(detour);

            if let Err(e) = unsafe { detour.enable() } {
                println!("{e}");
                return 1;
            }

            TRAMPOLINE
                .set(unsafe {
                    std::mem::transmute::<
                        &(),
                        unsafe extern "system" fn(*mut c_void, u32, DXGI_PRESENT) -> HRESULT,
                    >(detour.trampoline())
                })
                .unwrap();
        }
        Err(e) => {
            println!("{e}")
        }
    }

    0
}

static SHARED_BUFFER: OnceLock<ID3D11Texture2D> = OnceLock::new();

static TRAMPOLINE: OnceLock<unsafe extern "system" fn(*mut c_void, u32, DXGI_PRESENT) -> HRESULT> =
    OnceLock::new();

unsafe fn new_present_function(
    __this: *mut c_void,
    sync_internal: u32,
    flags: DXGI_PRESENT,
) -> HRESULT {
    if let Some(shared_buffer) = SHARED_BUFFER.get() {
        let this = IDXGISwapChain::from_raw(__this);

        let device: ID3D11Device = this.GetDevice().unwrap();
        let context = device.GetImmediateContext().unwrap();

        let texture: ID3D11Texture2D = this.GetBuffer(0).unwrap();

        static ONCE: Once = Once::new();

        ONCE.call_once(|| {
            let mut desc = D3D11_TEXTURE2D_DESC::default();

            texture.GetDesc(&mut desc);

            println!("{:?}", desc);
        });

        context.CopyResource(shared_buffer, &texture);
    } else {
        let this = IDXGISwapChain::from_raw(__this);

        let device: ID3D11Device1 = this.GetDevice().unwrap();

        match device.OpenSharedResourceByName::<_, ID3D11Texture2D>(
            w!("Sylvia's_Shared_Texture"),
            (DXGI_SHARED_RESOURCE_READ | DXGI_SHARED_RESOURCE_WRITE).0,
        ) {
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
    present_function(__this, sync_internal, flags)
}
