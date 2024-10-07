use windows::{
    core::{Interface, HRESULT},
    Win32::{
        Foundation::{HMODULE, HWND, RECT},
        Graphics::{
            Direct3D9::{
                D3D9b_SDK_VERSION, Direct3DCreate9, IDirect3DDevice9,
                D3DCREATE_HARDWARE_VERTEXPROCESSING, D3DDEVTYPE_HAL, D3DFMT_UNKNOWN,
                D3DMULTISAMPLE_NONE, D3DPRESENTFLAG_DEVICECLIP, D3DPRESENT_PARAMETERS,
                D3DSWAPEFFECT_COPY, D3DVIEWPORT9,
            },
            Gdi::RGNDATA,
        },
        UI::WindowsAndMessaging::WNDCLASSEXA,
    },
};

use crate::RenderingAPI;

pub struct DX9Hooks {
    window: HWND,
    window_class: WNDCLASSEXA,
    device: IDirect3DDevice9,
}

impl RenderingAPI for DX9Hooks {
    type PresentFn = unsafe extern "system" fn(
        *mut core::ffi::c_void,
        *const RECT,
        *const RECT,
        HWND,
        *const RGNDATA,
    ) -> HRESULT;

    type ResizeFn =
        unsafe extern "system" fn(*mut core::ffi::c_void, *const D3DVIEWPORT9) -> HRESULT;

    fn present_fn(&self) -> *const () {
        self.device.vtable().Present as _
    }

    fn resize_fn(&self) -> *const () {
        self.device.vtable().SetViewport as _
    }

    fn create(_: HMODULE) -> Result<Self, crate::error::Error> {
        let (window, window_class) = unsafe { super::create_window() }?;

        let d3d9 = unsafe { Direct3DCreate9(D3D9b_SDK_VERSION) }.unwrap();

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

        let device = device.unwrap();

        Ok(Self {
            window,
            window_class,
            device,
        })
    }

    fn destory(&self) -> Result<(), crate::error::Error> {
        unsafe {
            super::delete_window(self.window, self.window_class)?;
        }

        Ok(())
    }
}
