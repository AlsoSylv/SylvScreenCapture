use std::{mem::transmute, sync::OnceLock};

use retour::RawDetour;
use shared_defs::SharedMemoryHeader;
use windows::{
    Win32::{
        Foundation::{E_NOINTERFACE, HMODULE, HWND},
        Graphics::{
            Direct3D10::{
                D3D10_DRIVER_TYPE, D3D10_DRIVER_TYPE_HARDWARE, D3D10_SDK_VERSION, ID3D10Device,
                ID3D10Device1, ID3D10Texture2D,
            },
            Dxgi::{DXGI_SWAP_CHAIN_DESC, IDXGIAdapter, IDXGISwapChain},
        },
        System::LibraryLoader::GetProcAddress,
        UI::WindowsAndMessaging::WNDCLASSEXA,
    },
    core::{HRESULT, Interface, s},
};

use crate::{RenderingAPI, error::Error};

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

    fn create(module: HMODULE) -> Result<Self, Error> {
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

        let mut swap_chain_desc = super::dxgi_swap_chain_desc(window);

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

    fn set_detour(detour: RawDetour) {
        DETOUR.set(detour).unwrap()
    }
}

pub(super) fn dx10_new_present_fn(
    this: &IDXGISwapChain,
    header: &SharedMemoryHeader,
) -> Result<(), windows::core::Error> {
    static SHARED_BUFFER: OnceLock<ID3D10Texture2D> = OnceLock::new();

    // DX10 is not used
    if DETOUR.get().is_none() {
        return Err(windows::core::Error::new(E_NOINTERFACE, ""));
    }

    let device: ID3D10Device = unsafe { this.GetDevice() }?;
    header.set_api(shared_defs::RenderingAPI::Dx10);

    if let Some(shared_buffer) = SHARED_BUFFER.get() {
        // SAFETY: The back buffer is not dropped from this
        let back_buffer: ID3D10Texture2D = unsafe { this.GetBuffer(0) }.expect("Backbuffer exists");
        // SAFETY: This copies from the back buffer to the shared buffer
        unsafe { device.CopySubresourceRegion(shared_buffer, 0, 0, 0, 0, &back_buffer, 0, None) };
    } else {
        let handle = header.get_shared_handle();
        if let Some(handle) = handle {
            // SAFETY: This is always set later on in the code
            let mut texture = unsafe { std::mem::zeroed() };
            // This makes sure that the game does not crash if the shared handle cannot be opened, but there is probably a better way to do this.
            if let Err(e) = unsafe {
                device.OpenSharedResource(handle, &ID3D10Texture2D::IID, Some(&mut texture))
            } {
                println!("{e}");
                return Ok(());
            };

            // SAFETY: This is an ID3D10Texture2D, so this is safe
            let texture = unsafe { ID3D10Texture2D::from_raw(texture) };
            SHARED_BUFFER.get_or_init(|| texture);
        }
    }

    Ok(())
}
