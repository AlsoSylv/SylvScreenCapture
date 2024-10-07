use windows::{
    core::{s, PCSTR},
    Win32::{
        Foundation::HWND,
        System::LibraryLoader::GetModuleHandleA,
        UI::WindowsAndMessaging::{
            CreateWindowExA, DestroyWindow, RegisterClassExA, UnregisterClassA, CS_HREDRAW,
            CS_VREDRAW, WINDOW_EX_STYLE, WNDCLASSEXA, WS_OVERLAPPEDWINDOW,
        },
    },
};

use crate::error::Error;

pub use dx9::DX9Hooks;
pub use gl::OpenGLHooks;

mod dx9;
mod gl;

const WINDOW_CLASS_NAME: PCSTR = s!("dummy_window_for_swap_chain");

unsafe fn create_window() -> Result<(HWND, WNDCLASSEXA), Error> {
    let window_class = {
        let mut window_class = WNDCLASSEXA::default();
        window_class.cbSize = size_of::<WNDCLASSEXA>() as u32;
        window_class.style = CS_HREDRAW | CS_VREDRAW;
        window_class.lpfnWndProc = Some(crate::def_window_pro_a);
        window_class.hInstance = unsafe { GetModuleHandleA(None).unwrap().into() };
        window_class.lpszClassName = WINDOW_CLASS_NAME;
        window_class.lpszMenuName = PCSTR::null();
        window_class
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

    Ok((window, window_class))
}

unsafe fn delete_window(window: HWND, window_class: WNDCLASSEXA) -> Result<(), Error> {
    unsafe {
        DestroyWindow(window)?;
    }
    unsafe {
        UnregisterClassA(WINDOW_CLASS_NAME, window_class.hInstance)?;
    }

    Ok(())
}
