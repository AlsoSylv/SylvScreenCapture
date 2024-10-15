use std::{
    mem::transmute,
    sync::OnceLock,
};

use retour::RawDetour;
use windows::{
    core::{s, Interface, HRESULT},
    Win32::{
        Foundation::{BOOL, HMODULE, HWND},
        Graphics::{
            Direct3D10::{
                ID3D10Device, ID3D10Device1, ID3D10Texture2D, D3D10_DRIVER_TYPE,
                D3D10_DRIVER_TYPE_HARDWARE, D3D10_SDK_VERSION,
            },
            Dxgi::{
                Common::{
                    DXGI_FORMAT_R8G8B8A8_UNORM, DXGI_MODE_DESC, DXGI_MODE_SCALING_UNSPECIFIED,
                    DXGI_MODE_SCANLINE_ORDER_UNSPECIFIED, DXGI_RATIONAL, DXGI_SAMPLE_DESC,
                },
                IDXGIAdapter, IDXGISwapChain, DXGI_SWAP_CHAIN_DESC,
                DXGI_SWAP_CHAIN_FLAG_ALLOW_MODE_SWITCH, DXGI_SWAP_EFFECT_DISCARD,
                DXGI_USAGE_RENDER_TARGET_OUTPUT,
            },
        },
        System::LibraryLoader::GetProcAddress,
        UI::WindowsAndMessaging::WNDCLASSEXA,
    },
};

use crate::{error::Error, RenderingAPI};

static DETOUR: OnceLock<RawDetour> = OnceLock::new();

pub struct DX10Hooks {
    window: HWND,
    window_class: WNDCLASSEXA,
    swap_chain: IDXGISwapChain,
}

impl RenderingAPI for DX10Hooks {
    type PresentFn = super::PresentFn;

    type ResizeFn = super::ResizeFn;

    fn present_fn(&self) -> *const () {
        self.swap_chain.vtable().Present as _
    }

    fn resize_fn(&self) -> *const () {
        self.swap_chain.vtable().ResizeBuffers as _
    }

    fn create(module: HMODULE) -> Result<Self, crate::error::Error> {
        pub type D3D10CreateDeviceAndSwapChain = unsafe extern "system" fn(
            Option<IDXGIAdapter>,
            D3D10_DRIVER_TYPE,
            HMODULE,
            u32,
            u32,
            *mut DXGI_SWAP_CHAIN_DESC,
            *mut Option<IDXGISwapChain>,
            *mut Option<ID3D10Device1>,
        ) -> HRESULT;

        let (window, window_class) = unsafe { super::super::create_window() }?;

        let mut swap_chain_desc = DXGI_SWAP_CHAIN_DESC {
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
            Windowed: BOOL(true as i32),
            SwapEffect: DXGI_SWAP_EFFECT_DISCARD,
            Flags: DXGI_SWAP_CHAIN_FLAG_ALLOW_MODE_SWITCH.0 as u32,
        };

        let mut swap_chain = None;
        let mut device = None;

        let create_device =
            unsafe { GetProcAddress(module, s!("D3D10CreateDeviceAndSwapChain")).unwrap() };

        #[allow(non_snake_case)]
        let D3D10CreateDeviceAndSwapChain: D3D10CreateDeviceAndSwapChain =
            unsafe { transmute(create_device) };

        let result = unsafe {
            D3D10CreateDeviceAndSwapChain(
                None,
                D3D10_DRIVER_TYPE_HARDWARE,
                HMODULE::default(),
                0,
                D3D10_SDK_VERSION,
                &mut swap_chain_desc,
                &mut swap_chain,
                &mut device,
            )
        };

        if result.is_err() {
            return Err(windows::core::Error::from_hresult(result).into());
        }

        assert!(swap_chain.is_some());

        Ok(Self {
            swap_chain: swap_chain.unwrap(),
            window,
            window_class,
        })
    }

    fn destroy(&self) -> Result<(), Error> {
        unsafe { super::super::delete_window(self.window, self.window_class) }
    }

    fn trampoline() -> Self::PresentFn {
        *super::DXGI_SWAP_BUFFER.get().unwrap()
    }

    fn set_trampoline(func: Self::PresentFn) {
        super::DXGI_SWAP_BUFFER.get_or_init(|| func);
    }

    fn new_present_fn() -> Self::PresentFn {
        super::new_present_function
    }

    fn set_detour(detour: retour::RawDetour) {
        DETOUR.set(detour).unwrap()
    }
}

pub(super) fn dx10_new_present_fn(this: &IDXGISwapChain) {
    static SHARED_BUFFER: OnceLock<ID3D10Texture2D> = OnceLock::new();

    let _device: ID3D10Device = unsafe { this.GetDevice() }.unwrap();

    if let Some(_buffer) = SHARED_BUFFER.get() {
        // {
        //     static ONCE: Once = Once::new();
        //     ONCE.call_once(|| {
        //         let mut desc = D3D10_TEXTURE2D_DESC::default();
        //         unsafe { back_buffer.GetDesc(&mut desc) };
        //         desc.Usage = windows::Win32::Graphics::Direct3D10::D3D10_USAGE_STAGING;
        //         desc.BindFlags = 0;
        //         desc.CPUAccessFlags = D3D11_CPU_ACCESS_READ.0 as u32;
        //         let new_texture = unsafe { device_10.CreateTexture2D(&desc, None) }.unwrap();
        //         unsafe { device_10.Flush() };
        //         unsafe { device_10.CopyResource(&new_texture, &back_buffer) };
        //         unsafe { device_10.Flush() };
        //         let mapped_resource = unsafe { new_texture.Map(0, D3D10_MAP_READ, 0) }.unwrap();
        //         let slice = unsafe {
        //             std::slice::from_raw_parts(mapped_resource.pData as *const u8, 1920 * 1080 * 4)
        //         };
        //         let path = { PathBuf::from("C:\\Users\\Cynthia\\Pictures\\screenshot.png") };
        //         let file = BufWriter::new(std::fs::File::create(path).unwrap());
        //         let mut encoder = png::Encoder::new(file, 1920, 1080);
        //         encoder.set_color(png::ColorType::Rgba);
        //         encoder.set_depth(png::BitDepth::Eight);
        //         let mut writer = encoder.write_header().unwrap();
        //         writer
        //             .write_image_data(slice)
        //             .and_then(|()| writer.finish())
        //             .unwrap();
        //         println!("Captured");
        //         unsafe {
        //             new_texture.Unmap(0);
        //         }
        //     });
        // }
    } else {
        todo!("I need a shared handle that can work in DX10/9Ex, as well as the NT Handle for DX11/12")
    }
}
