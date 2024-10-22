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

use crate::{NewSharedMemoryHeader, RenderingAPI, SHARED_CPU_BUFFER};

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

#[repr(C)]
struct BGRA {
    b: u8,
    g: u8,
    r: u8,
    a: u8,
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
        let shared_memory_ptr = buffer.0.as_ptr();
        let header = unsafe { &*(shared_memory_ptr as *mut NewSharedMemoryHeader) };

        let back_buffer = unsafe { this.GetBackBuffer(0, 0, D3DBACKBUFFER_TYPE_MONO).unwrap() };
        let mut desc = D3DSURFACE_DESC::default();
        unsafe { back_buffer.GetDesc(&mut desc).unwrap() };

        let buffer_width = desc.Width;
        let buffer_width_usize = buffer_width as usize;
        let buffer_height = desc.Height;

        let mut out_surf = None;

        unsafe {
            this.CreateOffscreenPlainSurface(
                buffer_width,
                buffer_height,
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

        // The pitch is messured in bytes, so we need to divide by the size of BGRA
        let pitch = locked_rect.Pitch as usize / size_of::<BGRA>();
        let step_by = pitch * buffer_height as usize;

        {
            let (header_width, header_height) = header.get_width_and_height();
            assert!(header_width >= buffer_width, "Assert failed! Width of game buffer was larger than the capture buffer. Game buffer: {buffer_width}, Capture buffer: {header_width}");
            assert!(header_height >= buffer_height, "Assert failed! The number of rows in the game buffer was greater than the capture buffer. Game buffer: {buffer_height}, Capture buffer: {header_height}");
        }

        // SAFETY: The length of the buffer is equal to the pitch (padding) * the height * width
        let slice = unsafe {
            std::slice::from_raw_parts_mut(
                locked_rect.pBits.cast::<BGRA>(),
                step_by * buffer_width_usize,
            )
        };

        const HEADER_SIZE: usize = size_of::<NewSharedMemoryHeader>();

        header.set_width_and_height(buffer_width, buffer_height);
        header.set_api(crate::InUseRenderingAPI::Dx9);
        // SAFETY: The buffer allocated is after the header, but this will change in the future
        let buffer_ptr_head = unsafe { shared_memory_ptr.add(HEADER_SIZE) as *mut BGRA };
        for y in 0..buffer_height as usize {
            // When reading a DX9 mapped resource, it has a location inside the offset (in bytes), so that is calculated here
            let location = pitch * y;
            let slice = &mut slice[location..(location + buffer_width_usize)];

            // TODO: Ignore alpha in main process.
            // Simple way to encode the BGRA chunk to RGBA
            for bgra in slice.iter_mut() {
                let r = bgra.r;
                bgra.a = 255;
                bgra.r = bgra.b;
                bgra.b = r;
            }

            // SAFETY: The length of the buffer has to be at least the length of the frame buffer, and this should be asssured in the resize function
            // The Y here is the row of the buffer we're in
            // Since this is indexed as a 2D array, but an entire row is filled at a time, X is always 0
            let buffer_ptr = unsafe { buffer_ptr_head.add(0 + y * buffer_width_usize) };
            // SAFETY: This is copying the number of bytes equivalent less than or equal to the capture buffer length
            unsafe { buffer_ptr.copy_from(slice.as_ptr(), buffer_width_usize) };
        }

        unsafe { out_surf.UnlockRect().unwrap() }
    }

    let present_fn = DX9_PRESENT
        .get()
        .expect("The trampoline was set before this was ever called.");

    // SAFETY: This is the original present function to be called
    unsafe { (*present_fn)(this.as_raw(), src_rect, dst_rect, window, rgn) }
}
