use crate::error::Error;
use crate::{RenderingAPI, SHARED_HANDLE};
use retour::RawDetour;
use std::mem::transmute;
use std::ptr::NonNull;
use std::sync::atomic::Ordering;
use std::sync::OnceLock;
use windows::core::{s, Interface, HRESULT};
use windows::Win32::Foundation::{HANDLE, HMODULE, HWND};
use windows::Win32::Graphics::Direct3D::{
    D3D_DRIVER_TYPE, D3D_DRIVER_TYPE_HARDWARE, D3D_FEATURE_LEVEL,
};
use windows::Win32::Graphics::Direct3D11::{
    ID3D11Device, ID3D11Device1, ID3D11DeviceContext, ID3D11Texture2D, D3D11_SDK_VERSION,
};
use windows::Win32::Graphics::Dxgi::{IDXGIAdapter, IDXGISwapChain, DXGI_SWAP_CHAIN_DESC};
use windows::Win32::System::LibraryLoader::GetProcAddress;
use windows::Win32::UI::WindowsAndMessaging::WNDCLASSEXA;

static DETOUR: OnceLock<RawDetour> = OnceLock::new();

pub struct DX11Hooks {
    window: HWND,
    window_class: WNDCLASSEXA,
    swap_chain: IDXGISwapChain,
}

impl RenderingAPI for DX11Hooks {
    type PresentFn = super::PresentFn;
    type ResizeFn = super::ResizeFn;

    fn present_fn(&self) -> *const () {
        self.swap_chain.vtable().Present as _
    }

    fn resize_fn(&self) -> *const () {
        self.swap_chain.vtable().ResizeBuffers as _
    }

    fn create(module: HMODULE) -> Result<Self, Error> {
        pub type D3D11CreateDeviceAndSwapChain = unsafe extern "system" fn(
            Option<IDXGIAdapter>,
            D3D_DRIVER_TYPE,
            HMODULE,
            u32,
            Option<&D3D_FEATURE_LEVEL>,
            u32,
            u32,
            &DXGI_SWAP_CHAIN_DESC,
            &mut Option<IDXGISwapChain>,
            Option<&mut Option<ID3D11Device>>,
            Option<&mut D3D_FEATURE_LEVEL>,
            Option<&mut Option<ID3D11DeviceContext>>,
        ) -> HRESULT;

        let (window, window_class) = unsafe { super::super::create_window() }?;

        let swap_chain_desc = super::dxgi_swap_chain_desc(window);

        let mut swap_chain = None;

        let create_device =
            unsafe { GetProcAddress(module, s!("D3D11CreateDeviceAndSwapChain")).unwrap() };

        #[allow(non_snake_case)]
        let D3D11CreateDeviceAndSwapChain: D3D11CreateDeviceAndSwapChain =
            unsafe { transmute(create_device) };

        let result = unsafe {
            D3D11CreateDeviceAndSwapChain(
                None,
                D3D_DRIVER_TYPE_HARDWARE,
                HMODULE::default(),
                0,
                None,
                0,
                D3D11_SDK_VERSION,
                &swap_chain_desc,
                &mut swap_chain,
                None,
                None,
                None,
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

pub(super) fn dx11_duplicate_hook(this: &IDXGISwapChain) -> Result<(), windows::core::Error> {
    static SHARED_BUFFER: OnceLock<ID3D11Texture2D> = OnceLock::new();

    let device: ID3D11Device = unsafe { this.GetDevice() }?;
    let device_1: ID3D11Device1 = device
        .cast()
        .expect("Casting `ID3D11Device` to `ID3D11Device1` should never fail");

    if let Some(shared_buffer) = SHARED_BUFFER.get() {
        let context = unsafe { device.GetImmediateContext() }.expect("This is not null");
        let back_buffer: ID3D11Texture2D =
            unsafe { this.GetBuffer(0) }.expect("There's always a back buffer");

        // TODO: Find a better way of debugging
        // {
        //     static DESCRIPTION: Once = Once::new();
        //     DESCRIPTION.call_once(|| {
        //         let mut desc = D3D11_TEXTURE2D_DESC::default();
        //         unsafe { back_buffer.GetDesc(&mut desc) };
        //         let ty = unsafe { back_buffer.GetType() };
        //         println!("{desc:?} {ty:?}");
        //     });
        // }

        unsafe { context.CopyResource(shared_buffer, &back_buffer) };
    } else {
        // This sucks, but I don't think there's a better way to handle it.
        let handle = NonNull::new(SHARED_HANDLE.load(Ordering::Relaxed));
        let handle = handle.map(|ptr| HANDLE(ptr.as_ptr()));

        if let Some(handle) = handle {
            let maybe_shared_buffer = unsafe { device_1.OpenSharedResource1(handle) };
            if let Ok(shared_buffer) = maybe_shared_buffer {
                SHARED_BUFFER.get_or_init(|| shared_buffer);
            } else {
                println!("Error opening shared texture: {:?}", maybe_shared_buffer)
            }
        }
    }

    Ok(())
}
