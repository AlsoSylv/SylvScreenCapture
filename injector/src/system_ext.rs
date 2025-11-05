use std::{ffi::OsString, os::windows::ffi::OsStringExt};

use windows::{
    Win32::{
        Foundation::{HWND, LPARAM, SetLastError, WIN32_ERROR},
        Graphics::Dwm::{DWMWA_CLOAKED, DwmGetWindowAttribute},
        UI::{
            Input::KeyboardAndMouse::IsWindowEnabled,
            WindowsAndMessaging::{
                EnumWindows, GetWindowTextLengthW, GetWindowTextW, GetWindowThreadProcessId,
                IsWindowVisible,
            },
        },
    },
    core::{BOOL, Error},
};

macro_rules! try_win32 {
    ($win32_call:expr) => {{
        unsafe { SetLastError(WIN32_ERROR(0)) };
        let err = $win32_call;
        if err == 0 {
            let err = windows::core::Error::from_thread();
            if err.code().0 != 0 {
                unsafe { SetLastError(WIN32_ERROR(err.code().0 as u32)) };
                return false.into();
            }
        }

        err
    }};
}

pub struct System {}

type Callback<'a> = &'a mut dyn FnMut(OsString, u32, HWND) -> bool;
type CallbackPtr<'a> = *mut Callback<'a>;

impl System {
    pub fn new() -> Result<Self, Error> {
        Ok(Self {})
    }

    pub fn enum_windows(
        &self,
        mut window_callback: impl FnMut(OsString, u32, HWND) -> bool,
    ) -> Result<(), Error> {
        unsafe extern "system" fn enum_windows_callback(id: HWND, value: LPARAM) -> BOOL {
            const MAX_PATH: usize = windows::Win32::Foundation::MAX_PATH as usize;

            let callback: CallbackPtr = std::ptr::with_exposed_provenance_mut(value.0 as usize);

            if !unsafe { IsWindowVisible(id).as_bool() } {
                return true.into();
            }

            if !unsafe { IsWindowEnabled(id).into() } {
                return true.into();
            }

            let mut cloaked = BOOL::default();
            if let Err(e) = unsafe {
                DwmGetWindowAttribute(
                    id,
                    DWMWA_CLOAKED,
                    &raw mut cloaked as _,
                    size_of_val(&cloaked) as u32,
                )
            } {
                println!("{e}")
            }

            if cloaked.as_bool() {
                return true.into();
            }

            let mut process_id = 0;
            try_win32!(unsafe { GetWindowThreadProcessId(id, Some(&mut process_id)) });

            let title_len = try_win32!(unsafe { GetWindowTextLengthW(id) }) as usize;

            if title_len == 0 {
                return true.into();
            }

            let title_buffer: &mut [u16] = if title_len > MAX_PATH {
                &mut vec![0; title_len + 1]
            } else {
                &mut [0; MAX_PATH]
            };

            try_win32!(unsafe { GetWindowTextW(id, title_buffer) });

            let name = OsString::from_wide(&title_buffer[..title_len]);

            unsafe { (*callback)(name, process_id, id).into() }
        }

        let mut window_callback: Callback = &mut window_callback;
        let window_callback: CallbackPtr = &raw mut window_callback;

        unsafe {
            EnumWindows(
                Some(enum_windows_callback),
                LPARAM(window_callback.expose_provenance() as isize),
            )
        }
    }
}
