use std::ffi::CString;

use windows::{
    core::{s, PCSTR},
    Win32::{
        Foundation::{BOOL, HMODULE, HWND},
        Graphics::{
            Gdi::{GetDC, HDC},
            OpenGL::{
                wglCreateContext, wglDeleteContext, wglGetProcAddress, wglMakeCurrent,
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

        let context = unsafe { wglCreateContext(dc)? };

        unsafe { wglMakeCurrent(dc, context)? };

        glad_gl::gl::load(|func_ptr| {
            let c_string = CString::new(func_ptr).unwrap();
            let cstr = PCSTR::from_raw(c_string.as_ptr() as _);

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
        unsafe {
            wglMakeCurrent(self.dc, HGLRC::default())?;
        }
        unsafe {
            wglDeleteContext(self.ctx)?;
        }
        unsafe { super::delete_window(self.window, self.window_class)? }

        Ok(())
    }
}
