use std::{
    ffi::c_void,
    mem::transmute,
    ptr::{null, null_mut},
    sync::OnceLock,
};

use retour::RawDetour;
use windows::{
    core::{s, Interface, HRESULT},
    Win32::{
        Foundation::{HMODULE, HWND, RECT},
        Graphics::{
            Direct3D9::{
                D3D9b_SDK_VERSION, IDirect3D9, IDirect3DDevice9, D3DBACKBUFFER_TYPE_MONO,
                D3DCREATE_HARDWARE_VERTEXPROCESSING, D3DDEVTYPE_HAL, D3DFMT_UNKNOWN,
                D3DLOCKED_RECT, D3DLOCK_READONLY, D3DMULTISAMPLE_NONE, D3DPOOL_SYSTEMMEM,
                D3DPRESENTFLAG_DEVICECLIP, D3DPRESENT_PARAMETERS, D3DSURFACE_DESC,
                D3DSWAPEFFECT_COPY, D3DVIEWPORT9,
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

    fn create(module: HMODULE) -> Result<Self, crate::error::Error> {
        let (window, window_class) = unsafe { super::create_window() }?;

        type Direct3DCreate9 = unsafe extern "system" fn(u32) -> Option<IDirect3D9>;

        let direct_3d_create9_ptr =
            unsafe { GetProcAddress(module, s!("Direct3DCreate9")).unwrap() };

        #[allow(non_snake_case)]
        let Direct3DCreate9: Direct3DCreate9 = unsafe { transmute(direct_3d_create9_ptr) };

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

        let shared_buffer = shared_memory::ShmemConf::new()
            .os_id("SylvScreenShare")
            .size(size_of::<u32>() * 2 + size_of::<u32>() * 1920 * 1080)
            .open()
            .unwrap();

        SHARED_CPU_BUFFER.get_or_init(|| crate::SharedMem(shared_buffer));

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

    fn set_detour(detour: retour::RawDetour) {
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
            std::slice::from_raw_parts_mut(
                locked_rect.pBits.cast::<u8>(),
                step_by * width as usize * 4,
            )
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
                assert_eq!(slice.len(), 4);
                slice.swap(0, 2);
                slice[3] = 255;
            });

            let buffer_ptr = unsafe { buffer_ptr.add(4 * y * width as usize) };
            unsafe { buffer_ptr.copy_from(slice.as_ptr(), slice.len()) };
        }

        unsafe { out_surf.UnlockRect().unwrap() }
    }

    let present_fn = DX9_PRESENT
        .get()
        .expect("The trampoline was set before this was ever called.");

    // SAFETY: This is the original present function to be called
    unsafe { (*present_fn)(this.as_raw(), src_rect, dst_rect, window, rgn) }
}
