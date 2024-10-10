use crate::error::Error;
use crate::{RenderingAPI, SHARED_BUFFER, SHARED_HANDLE, WAS_OPENGL_CALL};
use retour::RawDetour;
use std::ffi::c_void;
use std::mem::transmute;
use std::ptr::{null, null_mut};
use std::sync::OnceLock;
use windows::core::{s, Interface, HRESULT};
use windows::Win32::Foundation::{BOOL, HANDLE, HMODULE, HWND};
use windows::Win32::Graphics::Direct3D::{
    D3D_DRIVER_TYPE, D3D_DRIVER_TYPE_HARDWARE, D3D_FEATURE_LEVEL,
};
use windows::Win32::Graphics::Direct3D11::{
    ID3D11Device, ID3D11Device1, ID3D11DeviceContext, ID3D11Texture2D, D3D11_SDK_VERSION,
};
use windows::Win32::Graphics::Dxgi::Common::{
    DXGI_FORMAT_R8G8B8A8_UNORM, DXGI_MODE_DESC, DXGI_MODE_SCALING_UNSPECIFIED,
    DXGI_MODE_SCANLINE_ORDER_UNSPECIFIED, DXGI_RATIONAL, DXGI_SAMPLE_DESC,
};
use windows::Win32::Graphics::Dxgi::{
    IDXGIAdapter, IDXGISwapChain, DXGI_PRESENT, DXGI_SWAP_CHAIN_DESC,
    DXGI_SWAP_CHAIN_FLAG_ALLOW_MODE_SWITCH, DXGI_SWAP_EFFECT_DISCARD,
    DXGI_USAGE_RENDER_TARGET_OUTPUT,
};
use windows::Win32::System::LibraryLoader::GetProcAddress;
use windows::Win32::UI::WindowsAndMessaging::WNDCLASSEXA;

static DXGI_SWAP_BUFFER: OnceLock<<DX11Hooks as RenderingAPI>::PresentFn> = OnceLock::new();
static DETOUR: OnceLock<RawDetour> = OnceLock::new();

type PresentFunctionType = unsafe extern "system" fn(*mut c_void, u32, DXGI_PRESENT) -> HRESULT;

pub struct DX11Hooks {
    window: HWND,
    window_class: WNDCLASSEXA,
    swap_chain: IDXGISwapChain,
}

impl RenderingAPI for DX11Hooks {
    type PresentFn = PresentFunctionType;
    type ResizeFn = ();

    fn present_fn(&self) -> *const () {
        self.swap_chain.vtable().Present as _
    }

    fn resize_fn(&self) -> *const () {
        self.swap_chain.vtable().ResizeBuffers as _
    }

    fn create(module: HMODULE) -> Result<Self, Error> {
        pub type D3D11CreateDeviceAndSwapChain = unsafe extern "system" fn(
            param0: Option<IDXGIAdapter>,
            D3D_DRIVER_TYPE,
            HMODULE,
            param3: u32,
            *const D3D_FEATURE_LEVEL,
            u32,
            u32,
            *const DXGI_SWAP_CHAIN_DESC,
            *mut Option<IDXGISwapChain>,
            *mut Option<ID3D11Device>,
            *mut D3D_FEATURE_LEVEL,
            *mut Option<ID3D11DeviceContext>,
        ) -> HRESULT;

        let (window, window_class) = unsafe { super::create_window() }?;

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

        let mut swap_chain = None;

        let create_device =
            unsafe { GetProcAddress(module, s!("D3D11CreateDeviceAndSwapChain")).unwrap() };

        #[allow(non_snake_case)]
        let D3D11CreateDeviceAndSwapChain: D3D11CreateDeviceAndSwapChain =
            unsafe { transmute(create_device) };

        unsafe {
            let result = D3D11CreateDeviceAndSwapChain(
                None,
                D3D_DRIVER_TYPE_HARDWARE,
                HMODULE::default(),
                0,
                null(),
                0,
                D3D11_SDK_VERSION,
                &swap_chain_desc,
                &mut swap_chain,
                null_mut(),
                null_mut(),
                null_mut(),
            );

            if result.is_err() {
                return Err(windows::core::Error::from_hresult(result).into());
            }
        };

        assert!(swap_chain.is_some());

        Ok(Self {
            swap_chain: swap_chain.unwrap(),
            window,
            window_class,
        })
    }

    fn destroy(&self) -> Result<(), Error> {
        unsafe { super::delete_window(self.window, self.window_class) }
    }

    fn trampoline() -> Self::PresentFn {
        *DXGI_SWAP_BUFFER.get().unwrap()
    }

    fn set_trampoline(func: Self::PresentFn) {
        DXGI_SWAP_BUFFER.set(func).unwrap()
    }

    fn new_present_fn() -> Self::PresentFn {
        new_present_function
    }

    fn set_detour(detour: retour::RawDetour) {
        DETOUR.set(detour).unwrap()
    }
}

// TODO: Make this a DXGI hook rather than DX11
// TODO: Check whether the device is DX10, DX11, or DX12
unsafe extern "system" fn new_present_function(
    this: *mut c_void,
    sync_internal: u32,
    flags: DXGI_PRESENT,
) -> HRESULT {
    let present_function = DXGI_SWAP_BUFFER
        .get()
        .expect("The trampoline was set before this was ever called.");

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

        // TODO: Find a better way of debugging
        // #[cfg(debug_assertions)]
        // {
        //     static DESCRIPTION: Once = Once::new();
        //     DESCRIPTION.call_once(|| {
        //         let mut desc = D3D11_TEXTURE2D_DESC::default();
        //         unsafe { back_buffer.GetDesc(&mut desc) };
        //         println!("{:?}", desc);
        //     });
        // }

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

    unsafe { present_function(this.as_raw(), sync_internal, flags) }
}
