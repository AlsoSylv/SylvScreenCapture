use std::{
    ffi::c_void,
    sync::{
        atomic::{AtomicBool, Ordering},
        OnceLock,
    },
};

use windows::Win32::Graphics::Dxgi::Common::{
    DXGI_FORMAT_R8G8B8A8_UNORM, DXGI_MODE_DESC, DXGI_MODE_SCALING_UNSPECIFIED,
    DXGI_MODE_SCANLINE_ORDER_UNSPECIFIED, DXGI_RATIONAL, DXGI_SAMPLE_DESC,
};
use windows::Win32::Graphics::Dxgi::{DXGI_SWAP_CHAIN_DESC, DXGI_USAGE_RENDER_TARGET_OUTPUT};
use windows::Win32::{Foundation::HWND, Graphics::Dxgi::DXGI_SWAP_EFFECT_FLIP_SEQUENTIAL};
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
pub mod dx12;

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

    // In case the driver or app are using DXGI presentation for OpenGL (though I'd rather do this through a DX device)

    let was_gl_call =
        WAS_OPENGL_CALL.compare_exchange(true, false, Ordering::SeqCst, Ordering::SeqCst);

    if let Err(false) = was_gl_call {
        // This is used in every capture (besides GL)
        let this = unsafe { IDXGISwapChain::from_raw_borrowed(&this).unwrap() };
        // This sucks, but I don't think there's a better way to handle it.
        let header = crate::SHARED_CPU_BUFFER.get().unwrap().0.header();

        // If this returns an `E_NOINTERFACE` error, that means that it is newer than DX10
        if let Err(e) = dx10::dx10_new_present_fn(this, header) {
            if e.code() == E_NOINTERFACE {
                // Repeat above but for DX11
                if let Err(e) = dx11::dx11_duplicate_hook(this, header) {
                    if e.code() == E_NOINTERFACE {
                        if let Err(e) = dx12::dx12_duplicate_hook(this, header) {
                            println!("{e}")
                        }
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
        BufferCount: 2,
        OutputWindow: window,
        Windowed: true.into(),
        SwapEffect: DXGI_SWAP_EFFECT_FLIP_SEQUENTIAL,
        Flags: 0,
    }
}
