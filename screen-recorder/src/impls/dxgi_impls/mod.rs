use std::{
    ffi::c_void,
    sync::{
        atomic::{AtomicBool, Ordering},
        OnceLock,
    },
};

use windows::{
    core::{Interface, HRESULT},
    Win32::{
        Foundation::E_NOINTERFACE,
        Graphics::Dxgi::{Common::DXGI_FORMAT, IDXGISwapChain, DXGI_PRESENT},
    },
};

pub(super) static WAS_OPENGL_CALL: AtomicBool = AtomicBool::new(false);

pub mod dx10;
pub mod dx11;

type PresentFn = unsafe extern "system" fn(*mut c_void, u32, DXGI_PRESENT) -> HRESULT;
type ResizeFn = unsafe extern "system" fn(*mut c_void, u32, u32, u32, DXGI_FORMAT, u32) -> HRESULT;

static DXGI_SWAP_BUFFER: OnceLock<PresentFn> = OnceLock::new();

// TODO: Make this a DXGI hook rather than DX11
// TODO: Check whether the device is DX10, DX11, or DX12
unsafe extern "system" fn new_present_function(
    this: *mut c_void,
    sync_internal: u32,
    flags: DXGI_PRESENT,
) -> HRESULT {
    const GET_PRESENT_ERROOR: &str = "The trampoline was set before this was ever called.";
    let present_function = *DXGI_SWAP_BUFFER.get().expect(GET_PRESENT_ERROOR);

    if WAS_OPENGL_CALL.load(Ordering::SeqCst) {
        WAS_OPENGL_CALL.store(false, Ordering::SeqCst);
    } else {
        let this = unsafe { IDXGISwapChain::from_raw(this) };

        if let Err(e) = dx10::dx10_new_present_fn(&this) {
            if e.code() == E_NOINTERFACE {
                if let Err(e) = dx11::dx11_duplicate_hook(&this) {
                    if e.code() == E_NOINTERFACE {
                        todo!("D3D12 Call here");
                    }
                }
            } else {
                println!("{e:?}")
            }
        }
    }

    unsafe { present_function(this, sync_internal, flags) }
}
