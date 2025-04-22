use std::{ffi::c_void, mem::transmute, sync::OnceLock};

use retour::RawDetour;
use windows::{
    core::{s, Interface, HRESULT},
    Win32::{
        Foundation::{HMODULE, HWND, RECT},
        Graphics::{
            Direct3D9::{
                D3D9b_SDK_VERSION, IDirect3D9, IDirect3DDevice9, D3DBACKBUFFER_TYPE_MONO,
                D3DCREATE_HARDWARE_VERTEXPROCESSING, D3DDEVTYPE_HAL, D3DFMT_A8R8G8B8,
                D3DFMT_UNKNOWN, D3DMULTISAMPLE_NONE, D3DPOOL_DEFAULT, D3DPRESENTFLAG_DEVICECLIP,
                D3DPRESENT_PARAMETERS, D3DSURFACE_DESC, D3DSWAPEFFECT_COPY, D3DUSAGE_RENDERTARGET,
                D3DVIEWPORT9,
            },
            Gdi::RGNDATA,
        },
        System::LibraryLoader::GetProcAddress,
        UI::WindowsAndMessaging::WNDCLASSEXA,
    },
};

static DX9_PRESENT: OnceLock<<DX9Hooks as RenderingAPI>::PresentFn> = OnceLock::new();
static DETOUR: OnceLock<RawDetour> = OnceLock::new();

use crate::{RenderingAPI, SHARED_CPU_BUFFER};

pub struct DX9Hooks {
    window: HWND,
    window_class: WNDCLASSEXA,
    device: IDirect3DDevice9,
}

impl RenderingAPI for DX9Hooks {
    type PresentFn = unsafe extern "system" fn(
        *mut c_void,
        *const RECT,
        *const RECT,
        HWND,
        *const RGNDATA,
    ) -> HRESULT;

    type ResizeFn = unsafe extern "system" fn(*mut c_void, *const D3DVIEWPORT9) -> HRESULT;

    fn present_fn(&self) -> *const () {
        self.device.vtable().Present as _
    }

    fn resize_fn(&self) -> *const () {
        self.device.vtable().SetViewport as _
    }

    fn create(module: HMODULE) -> Result<Self, crate::error::Error> {
        let (window, window_class) = unsafe { super::create_window() }?;

        type Direct3DCreate9 = unsafe extern "system" fn(u32) -> Option<IDirect3D9>;

        let direct_3d_create9_ptr =
            unsafe { GetProcAddress(module, s!("Direct3DCreate9")).unwrap() };

        #[allow(non_snake_case)]
        let Direct3DCreate9: Direct3DCreate9 = unsafe { transmute(direct_3d_create9_ptr) };

        let Some(d3d9) = (unsafe { Direct3DCreate9(D3D9b_SDK_VERSION) }) else {
            return Err(
                windows::core::Error::from_hresult(windows::Win32::Foundation::E_FAIL).into(),
            );
        };

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

    fn destroy(&self) -> Result<(), crate::error::Error> {
        unsafe {
            super::delete_window(self.window, self.window_class)?;
        }

        Ok(())
    }

    fn trampoline() -> Self::PresentFn {
        *DX9_PRESENT.get().unwrap()
    }

    fn set_trampoline(func: Self::PresentFn) {
        DX9_PRESENT.set(func).unwrap()
    }

    fn new_present_fn() -> Self::PresentFn {
        new_dx9_present_function
    }

    fn set_detour(detour: RawDetour) {
        DETOUR.set(detour).unwrap()
    }
}

unsafe extern "system" fn new_dx9_present_function(
    this: *mut c_void,
    src_rect: *const RECT,
    dst_rect: *const RECT,
    window: HWND,
    rgn: *const RGNDATA,
) -> HRESULT {
    let this = unsafe { IDirect3DDevice9::from_raw_borrowed(&this).unwrap() };

    if let Some(buffer) = SHARED_CPU_BUFFER.get() {
        let header = buffer.0.as_ref();
        header.set_api(shmem::RenderingAPI::Dx9);
        header.set_width_and_height(1920, 1080);

        let Some(mut handle) = header.get_shared_handle() else {
            let present_fn = DX9_PRESENT
                .get()
                .expect("The trampoline was set before this was ever called.");

            // SAFETY: This is the original present function to be called
            return unsafe { (*present_fn)(this.as_raw(), src_rect, dst_rect, window, rgn) };
        };

        let back_buffer = unsafe { this.GetBackBuffer(0, 0, D3DBACKBUFFER_TYPE_MONO) }.unwrap();
        let mut desc = D3DSURFACE_DESC::default();
        unsafe { back_buffer.GetDesc(&mut desc) }.unwrap();

        let buffer_width = desc.Width;
        let buffer_height = desc.Height;

        let mut out_surf = None;

        unsafe {
            this.CreateTexture(
                buffer_width,
                buffer_height,
                1,
                D3DUSAGE_RENDERTARGET as u32,
                D3DFMT_A8R8G8B8,
                D3DPOOL_DEFAULT,
                &mut out_surf,
                &mut handle,
            )
        }
        .unwrap();

        let out_surf = out_surf.unwrap();

        let surf = out_surf.GetSurfaceLevel(0).unwrap();

        if let Err(e) = unsafe { this.GetRenderTargetData(&back_buffer, &surf) } {
            println!("{e:?}")
        };
    }

    let present_fn = DX9_PRESENT
        .get()
        .expect("The trampoline was set before this was ever called.");

    // SAFETY: This is the original present function to be called
    unsafe { (*present_fn)(this.as_raw(), src_rect, dst_rect, window, rgn) }
}
