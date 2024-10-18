use std::{
    ffi::c_void,
    sync::{
        atomic::{AtomicBool, Ordering},
        OnceLock,
    },
};

use windows::Win32::Foundation::HWND;
use windows::Win32::Graphics::Dxgi::Common::{
    DXGI_FORMAT_R8G8B8A8_UNORM, DXGI_MODE_DESC, DXGI_MODE_SCALING_UNSPECIFIED,
    DXGI_MODE_SCANLINE_ORDER_UNSPECIFIED, DXGI_RATIONAL, DXGI_SAMPLE_DESC,
};
use windows::Win32::Graphics::Dxgi::{
    DXGI_SWAP_CHAIN_DESC, DXGI_SWAP_CHAIN_FLAG_ALLOW_MODE_SWITCH, DXGI_SWAP_EFFECT_DISCARD,
    DXGI_USAGE_RENDER_TARGET_OUTPUT,
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
    const GET_PRESENT_ERROR: &str = "The trampoline was set before this was ever called.";
    let present_function = *DXGI_SWAP_BUFFER.get().expect(GET_PRESENT_ERROR);

    // Because drivers inject GL calls (
    if WAS_OPENGL_CALL.load(Ordering::SeqCst) {
        WAS_OPENGL_CALL.store(false, Ordering::SeqCst);
    } else {
        // This is used in every capture (besides GL)
        let this = unsafe { IDXGISwapChain::from_raw(this) };

        // If this returns an `E_NOINTERFACE` error, that means that it is newer than DX10
        if let Err(e) = dx10::dx10_new_present_fn(&this) {
            if e.code() == E_NOINTERFACE {
                // Repeat above but for DX11
                if let Err(e) = dx11::dx11_duplicate_hook(&this) {
                    if e.code() == E_NOINTERFACE {
                        todo!("D3D12 Call here");
                    }
                }
            } else {
                // Any other sort of error *probably* has to be handled, but I haven't seen them
                println!("{e:?}")
            }
        }
    }

    unsafe { present_function(this, sync_internal, flags) }
}

fn dxgi_swap_chain_desc(window: HWND) -> DXGI_SWAP_CHAIN_DESC {
    DXGI_SWAP_CHAIN_DESC {
        BufferDesc: DXGI_MODE_DESC {
            Width: 100,
            Height: 100,
            RefreshRate: DXGI_RATIONAL {
                Numerator: 60,
                Denominator: 1,
            },
            Format: DXGI_FORMAT_R8G8B8A8_UNORM,
            ScanlineOrdering: DXGI_MODE_SCANLINE_ORDER_UNSPECIFIED,
            Scaling: DXGI_MODE_SCALING_UNSPECIFIED,
        },
        SampleDesc: DXGI_SAMPLE_DESC {
            Count: 1,
            Quality: 0,
        },
        BufferUsage: DXGI_USAGE_RENDER_TARGET_OUTPUT,
        BufferCount: 1,
        OutputWindow: window,
        Windowed: true.into(),
        SwapEffect: DXGI_SWAP_EFFECT_DISCARD,
        Flags: DXGI_SWAP_CHAIN_FLAG_ALLOW_MODE_SWITCH.0 as u32,
    }
}
