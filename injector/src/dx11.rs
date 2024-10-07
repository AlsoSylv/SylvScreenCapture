use windows::Win32::Foundation::HWND;
use windows::Win32::Graphics::Direct3D::{
    D3D_DRIVER_TYPE_UNKNOWN, D3D_FEATURE_LEVEL, D3D_FEATURE_LEVEL_11_1,
};
use windows::Win32::Graphics::Direct3D11::{
    D3D11CreateDeviceAndSwapChain, ID3D11Device, ID3D11DeviceContext, D3D11_CREATE_DEVICE_DEBUG,
    D3D11_SDK_VERSION,
};
use windows::Win32::Graphics::Dxgi::Common::{
    DXGI_FORMAT_R8G8B8A8_UNORM_SRGB, DXGI_FORMAT_UNKNOWN, DXGI_MODE_DESC,
    DXGI_MODE_SCALING_UNSPECIFIED, DXGI_MODE_SCANLINE_ORDER_UNSPECIFIED, DXGI_RATIONAL,
    DXGI_SAMPLE_DESC,
};
use windows::Win32::Graphics::Dxgi::{
    CreateDXGIFactory, IDXGIFactory, IDXGISwapChain, DXGI_MWA_NO_ALT_ENTER, DXGI_SWAP_CHAIN_DESC,
    DXGI_SWAP_CHAIN_FLAG, DXGI_SWAP_EFFECT_DISCARD, DXGI_USAGE_RENDER_TARGET_OUTPUT,
};
use winit::raw_window_handle::Win32WindowHandle;

pub fn create_device_and_swap_chain(
    width: u32,
    height: u32,
    window: &Win32WindowHandle,
) -> (IDXGISwapChain, ID3D11Device, ID3D11DeviceContext) {
    let dxgi_factory: IDXGIFactory = unsafe { CreateDXGIFactory().unwrap() };
    let adapters = unsafe { dxgi_factory.EnumAdapters(0).unwrap() };

    let swap_chain_desc = DXGI_SWAP_CHAIN_DESC {
        BufferDesc: DXGI_MODE_DESC {
            Width: width,
            Height: height,
            RefreshRate: DXGI_RATIONAL {
                Numerator: 60,
                Denominator: 1,
            },
            Format: DXGI_FORMAT_R8G8B8A8_UNORM_SRGB,
            ScanlineOrdering: DXGI_MODE_SCANLINE_ORDER_UNSPECIFIED,
            Scaling: DXGI_MODE_SCALING_UNSPECIFIED,
        },
        SampleDesc: DXGI_SAMPLE_DESC {
            Count: 1,
            Quality: 0,
        },
        BufferUsage: DXGI_USAGE_RENDER_TARGET_OUTPUT,
        BufferCount: 2,
        OutputWindow: HWND(window.hwnd.get() as _),
        Windowed: true.into(),
        SwapEffect: DXGI_SWAP_EFFECT_DISCARD,
        Flags: 0,
    };

    let mut device = None;
    let mut swap_chain = None;
    let mut context = None;
    let mut d3d_feature_level = D3D_FEATURE_LEVEL::default();

    unsafe {
        D3D11CreateDeviceAndSwapChain(
            &adapters,
            D3D_DRIVER_TYPE_UNKNOWN,
            None,
            D3D11_CREATE_DEVICE_DEBUG,
            Some(&[D3D_FEATURE_LEVEL_11_1]),
            D3D11_SDK_VERSION,
            Some(&swap_chain_desc),
            Some(&mut swap_chain),
            Some(&mut device),
            Some(&mut d3d_feature_level),
            Some(&mut context),
        )
        .unwrap();
    };

    unsafe {
        dxgi_factory
            .MakeWindowAssociation(HWND(window.hwnd.get() as _), DXGI_MWA_NO_ALT_ENTER)
            .unwrap();
    }

    (swap_chain.unwrap(), device.unwrap(), context.unwrap())
}

pub fn resize_back_buffer(
    swap_chain: &IDXGISwapChain,
    new_width: u32,
    new_height: u32,
) -> Result<(), windows::core::Error> {
    unsafe {
        swap_chain.ResizeBuffers(
            0,
            new_width,
            new_height,
            DXGI_FORMAT_UNKNOWN,
            DXGI_SWAP_CHAIN_FLAG(0),
        )
    }
}
