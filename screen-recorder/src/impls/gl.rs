use std::{ffi::CString, mem::transmute};

use windows::{
    core::{s, PCSTR},
    Win32::{
        Foundation::{BOOL, HMODULE, HWND},
        Graphics::{
            Gdi::{GetDC, HDC},
            OpenGL::{
                ChoosePixelFormat, SetPixelFormat, HGLRC, PFD_DOUBLEBUFFER, PFD_DRAW_TO_WINDOW,
                PFD_MAIN_PLANE, PFD_SUPPORT_OPENGL, PFD_TYPE_RGBA, PIXELFORMATDESCRIPTOR,
            },
        },
        System::LibraryLoader::GetProcAddress,
        UI::WindowsAndMessaging::WNDCLASSEXA,
    },
};

use crate::RenderingAPI;

type WglSwapBuffers = unsafe extern "system" fn(HDC) -> BOOL;
type GlViewPort = unsafe fn(i32, i32, i32, i32);

pub struct OpenGLHooks {
    module: HMODULE,
    window: HWND,
    window_class: WNDCLASSEXA,
    ctx: HGLRC,
    dc: HDC,
}

impl RenderingAPI for OpenGLHooks {
    type PresentFn = WglSwapBuffers;
    type ResizeFn = GlViewPort;

    fn present_fn(&self) -> *const () {
        const SWAP: windows::core::PCSTR = s!("wglSwapBuffers");

        let func = unsafe { GetProcAddress(self.module, SWAP).unwrap() };
        func as _
    }

    fn resize_fn(&self) -> *const () {
        const VIEWPORT: PCSTR = s!("glViewport");

        let func = unsafe { GetProcAddress(self.module, VIEWPORT).unwrap() };
        func as _
    }

    fn create(module: HMODULE) -> Result<Self, crate::error::Error>
    where
        Self: Sized,
    {
        let (window, window_class) = unsafe { super::create_window() }?;

        let dc = unsafe { GetDC(window) };

        let pixel_format = {
            let mut pixel_format = PIXELFORMATDESCRIPTOR::default();
            pixel_format.nSize = size_of::<PIXELFORMATDESCRIPTOR>() as u16;
            pixel_format.nVersion = 1;
            pixel_format.dwFlags = PFD_DRAW_TO_WINDOW | PFD_SUPPORT_OPENGL | PFD_DOUBLEBUFFER;
            pixel_format.iPixelType = PFD_TYPE_RGBA;
            pixel_format.cDepthBits = 24;
            pixel_format.cStencilBits = 8;
            pixel_format.iLayerType = PFD_MAIN_PLANE.0 as u8;
            pixel_format
        };

        let pixel_format_idx = unsafe { ChoosePixelFormat(dc, &pixel_format) };

        unsafe { SetPixelFormat(dc, pixel_format_idx, &pixel_format)? };

        type PROC = Option<unsafe extern "system" fn() -> isize>;
        type WglCreateContext = unsafe extern "system" fn(HDC) -> HGLRC;
        type WglMakeCurrent = unsafe extern "system" fn(HDC, HGLRC) -> BOOL;
        type WglGetProcAddress = unsafe extern "system" fn(PCSTR) -> PROC;

        let wgl_create_context_ptr =
            unsafe { GetProcAddress(module, s!("wglCreateContext")).unwrap() };
        let wgl_make_current_ptr = unsafe { GetProcAddress(module, s!("wglMakeCurrent")).unwrap() };
        let wgl_get_proc_address_ptr =
            unsafe { GetProcAddress(module, s!("wglGetProcAddress")).unwrap() };

        #[allow(non_snake_case)]
        let wglCreateContext: WglCreateContext = unsafe { transmute(wgl_create_context_ptr) };
        #[allow(non_snake_case)]
        let wglMakeCurrent: WglMakeCurrent = unsafe { transmute(wgl_make_current_ptr) };
        #[allow(non_snake_case)]
        let wglGetProcAddress: WglGetProcAddress = unsafe { transmute(wgl_get_proc_address_ptr) };

        let context = unsafe { wglCreateContext(dc) };

        if context.is_invalid() {
            return Err(windows::core::Error::from_win32().into());
        }

        unsafe { wglMakeCurrent(dc, context).ok()? };

        glad_gl::gl::load(|func_ptr| {
            let c_string = CString::new(func_ptr).unwrap();
            let cstr = PCSTR(c_string.as_ptr() as _);

            let func_ptr = unsafe { wglGetProcAddress(cstr) };

            if let Some(func_ptr) = func_ptr {
                func_ptr as _
            } else {
                unsafe {
                    GetProcAddress(module, cstr).map_or_else(|| std::ptr::null(), |func| func as _)
                }
            }
        });

        Ok(Self {
            module,
            window,
            ctx: context,
            window_class,
            dc,
        })
    }

    fn destory(&self) -> Result<(), crate::error::Error> {
        type WglMakeCurrent = unsafe extern "system" fn(HDC, HGLRC) -> BOOL;
        type WglDeleteContext = unsafe extern "system" fn(HGLRC) -> BOOL;

        let wgl_make_current_ptr =
            unsafe { GetProcAddress(self.module, s!("wglMakeCurrent")).unwrap() };
        let wgl_delete_context_ptr =
            unsafe { GetProcAddress(self.module, s!("wglDeleteContext")).unwrap() };

        #[allow(non_snake_case)]
        let wglMakeCurrent: WglMakeCurrent = unsafe { transmute(wgl_make_current_ptr) };
        #[allow(non_snake_case)]
        let wglDeleteContext: WglDeleteContext = unsafe { transmute(wgl_delete_context_ptr) };

        unsafe {
            wglMakeCurrent(self.dc, HGLRC::default()).ok()?;
        }
        unsafe {
            wglDeleteContext(self.ctx).ok()?;
        }
        unsafe { super::delete_window(self.window, self.window_class)? }

        Ok(())
    }
}
