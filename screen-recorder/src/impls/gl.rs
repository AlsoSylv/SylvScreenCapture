use std::{mem::transmute, ptr::addr_of_mut, sync::OnceLock};

use retour::RawDetour;
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

static OPENGL_SWAP_BUFFERS: OnceLock<<OpenGLHooks as RenderingAPI>::PresentFn> = OnceLock::new();

static DETOUR: OnceLock<RawDetour> = OnceLock::new();

pub struct OpenGLHooks {
    module: HMODULE,
    window: HWND,
    window_class: WNDCLASSEXA,
    ctx: HGLRC,
    dc: HDC,
}

impl RenderingAPI for OpenGLHooks {
    type PresentFn = unsafe extern "system" fn(HDC) -> BOOL;
    type ResizeFn = unsafe fn(i32, i32, i32, i32);

    fn present_fn(&self) -> *const () {
        // Get the `wglSwapBuffers` function and return it as the `present` function
        const SWAP: PCSTR = s!("wglSwapBuffers");
        let func = unsafe { GetProcAddress(self.module, SWAP).unwrap() };
        func as _
    }

    fn resize_fn(&self) -> *const () {
        // Get the `glViewport` function and return it as the `resize` function
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

        let pixel_format = PIXELFORMATDESCRIPTOR {
            nSize: size_of::<PIXELFORMATDESCRIPTOR>() as u16,
            nVersion: 1,
            dwFlags: PFD_DRAW_TO_WINDOW | PFD_SUPPORT_OPENGL | PFD_DOUBLEBUFFER,
            iPixelType: PFD_TYPE_RGBA,
            cDepthBits: 24,
            cStencilBits: 8,
            iLayerType: PFD_MAIN_PLANE.0 as u8,
            ..Default::default()
        };

        let pixel_format_idx = unsafe { ChoosePixelFormat(dc, &pixel_format) };

        unsafe { SetPixelFormat(dc, pixel_format_idx, &pixel_format)? };

        // God cannot save me from my sins.
        type Proc = Option<unsafe extern "system" fn() -> isize>;
        type WglCreateContext = unsafe extern "system" fn(HDC) -> HGLRC;
        type WglMakeCurrent = unsafe extern "system" fn(HDC, HGLRC) -> BOOL;
        type WglGetProcAddress = unsafe extern "system" fn(PCSTR) -> Proc;

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

        // Load all the GL function pointers from GLAD, this will let us use it later in the `present` hook
        glad_gl::gl::load(|func_name| {
            // They are all ascii anyway
            debug_assert!(func_name.is_ascii());
            let null_terminated =
                std::ffi::CString::new(func_name).expect("There's no null in them");
            let cast_fn = |func| func as _;
            let cstr = PCSTR(null_terminated.as_ptr() as _);
            let func_ptr = unsafe { wglGetProcAddress(cstr) };

            func_ptr.map_or_else(
                || unsafe { GetProcAddress(module, cstr).map_or_else(std::ptr::null, cast_fn) },
                cast_fn,
            )
        });

        Ok(Self {
            module,
            window,
            ctx: context,
            window_class,
            dc,
        })
    }

    fn destroy(&self) -> Result<(), crate::error::Error> {
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

    fn trampoline() -> Self::PresentFn {
        *OPENGL_SWAP_BUFFERS.get().unwrap()
    }

    fn set_trampoline(func: Self::PresentFn) {
        OPENGL_SWAP_BUFFERS.set(func).unwrap()
    }

    fn new_present_fn() -> Self::PresentFn {
        new_wgl_swap_buffers
    }

    fn set_detour(detour: RawDetour) {
        DETOUR.set(detour).unwrap()
    }
}

unsafe extern "system" fn new_wgl_swap_buffers(un_named_1: HDC) -> BOOL {
    use glad_gl::gl;

    #[cfg(target_os = "windows")]
    {
        use super::dxgi_impls::WAS_OPENGL_CALL;
        WAS_OPENGL_CALL.store(true, std::sync::atomic::Ordering::SeqCst);
    }

    let lock = crate::SHARED_CPU_BUFFER.read().unwrap();
    let header = lock.as_ref().unwrap().as_ref();
    header.set_api(shmem::RenderingAPI::Ogl);
    let handle = header.get_nt_shared_handle();

    if let Some(handle) = handle {
        let (mut memory_object, mut texture) = (0, 0);

        unsafe { gl::CreateMemoryObjectsEXT(1, addr_of_mut!(memory_object)) };
        // TODO: This should be replaced with `ImportMemoryFd` on Linux
        unsafe {
            gl::ImportMemoryWin32HandleEXT(
                memory_object,
                0,
                gl::HANDLE_TYPE_D3D11_IMAGE_EXT,
                handle.0,
            )
        };

        unsafe { gl::GenTextures(1, addr_of_mut!(texture)) };
        unsafe { gl::BindTexture(gl::TEXTURE_2D, texture) };

        unsafe {
            gl::TexStorageMem2DEXT(gl::TEXTURE_2D, 1, gl::RGBA8, 1920, 1080, memory_object, 0)
        };
        unsafe { gl::CopyTexSubImage2D(gl::TEXTURE_2D, 0, 0, 0, 0, 0, 1920, 1080) };
        unsafe { gl::DeleteTextures(1, &texture) };
        unsafe { gl::DeleteMemoryObjectsEXT(1, &memory_object) };
    }

    let present_function = OPENGL_SWAP_BUFFERS
        .get()
        .expect("The trampoline was set before this was ever called.");

    unsafe { present_function(un_named_1) }
}
