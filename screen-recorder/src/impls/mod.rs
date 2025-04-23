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
pub use dxgi_impls::dx10::DX10Hooks;
pub use dxgi_impls::dx11::DX11Hooks;
pub use dxgi_impls::dx12::DX12Hooks;
pub use gl::OpenGLHooks;
pub use vk::VkHooks;

mod dx9;
mod dxgi_impls;
mod gl;
mod vk;

const WINDOW_CLASS_NAME: PCSTR = s!("dummy_window_for_swap_chain");

unsafe fn create_window() -> Result<(HWND, WNDCLASSEXA), Error> {
    let window_class = WNDCLASSEXA {
        cbSize: size_of::<WNDCLASSEXA>() as u32,
        style: CS_HREDRAW | CS_VREDRAW,
        lpfnWndProc: Some(def_window_pro_a),
        hInstance: unsafe { GetModuleHandleA(None)?.into() },
        lpszClassName: WINDOW_CLASS_NAME,
        lpszMenuName: PCSTR::null(),
        ..Default::default()
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
            10,
            10,
            100,
            100,
            None,
            None,
            Some(window_class.hInstance),
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
        UnregisterClassA(WINDOW_CLASS_NAME, Some(window_class.hInstance))?;
    }

    Ok(())
}

// Workaround
// The windows crate `DefWindowProcA` is not an extern "system" function,
// So it cannot be passed to `WNDCLASSEXA` and needs to be put in a wrapper to do so
unsafe extern "system" fn def_window_pro_a(
    hwnd: HWND,
    msg: u32,
    wparam: windows::Win32::Foundation::WPARAM,
    lparam: windows::Win32::Foundation::LPARAM,
) -> windows::Win32::Foundation::LRESULT {
    windows::Win32::UI::WindowsAndMessaging::DefWindowProcA(hwnd, msg, wparam, lparam)
}
