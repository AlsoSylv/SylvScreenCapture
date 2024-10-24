use std::{
    ffi::c_void,
    ptr::{null, null_mut},
    sync::{atomic::AtomicPtr, OnceLock},
};

use shmem::SharedMemoryHeader;
use windows::{
    core::{s, Interface},
    Win32::{
        Foundation::HWND,
        Graphics::{
            Direct3D::D3D_FEATURE_LEVEL,
            Direct3D12::{
                ID3D12CommandAllocator, ID3D12CommandList, ID3D12CommandQueue, ID3D12Device,
                ID3D12GraphicsCommandList, ID3D12Resource, D3D12_COMMAND_LIST_TYPE_COPY,
            },
            Dxgi::IDXGISwapChain,
        },
        System::LibraryLoader::GetProcAddress,
        UI::WindowsAndMessaging::WNDCLASSEXA,
    },
};

use crate::RenderingAPI;

pub struct DX12Hooks {
    window: HWND,
    window_class: WNDCLASSEXA,
    swap_chain: IDXGISwapChain,
}

#[allow(unused)]
impl RenderingAPI for DX12Hooks {
    type PresentFn = super::PresentFn;

    type ResizeFn = super::ResizeFn;

    fn present_fn(&self) -> *const () {
        self.swap_chain.vtable().Present as _
    }

    fn resize_fn(&self) -> *const () {
        self.swap_chain.vtable().ResizeBuffers as _
    }

    fn create(module: windows::Win32::Foundation::HMODULE) -> Result<Self, crate::error::Error> {
        #[allow(non_snake_case)]
        pub type D3D12CreateDevice = unsafe extern "system" fn(
            param0: Option<windows::core::IUnknown>,
            param1: D3D_FEATURE_LEVEL,
            param2: *const windows::core::GUID,
            param3: *mut Option<ID3D12Device>,
        ) -> windows::core::HRESULT;

        let d3d12_create_device_ptr =
            unsafe { GetProcAddress(module, s!("D3D12CreateDevice")) }.unwrap();

        #[allow(non_snake_case)]
        let D3D12CreateDevice: D3D12CreateDevice =
            unsafe { std::mem::transmute(d3d12_create_device_ptr) };

        let mut device = None;
        let result = unsafe {
            D3D12CreateDevice(None, D3D_FEATURE_LEVEL(0), &ID3D12Device::IID, &mut device)
        };

        if result.is_err() {
            Err(windows::core::Error::from_hresult(result))?;
        }

        let device = device.unwrap();

        todo!()
    }

    fn destroy(&self) -> Result<(), crate::error::Error> {
        unsafe { super::super::delete_window(self.window, self.window_class) }
    }

    fn trampoline() -> Self::PresentFn {
        todo!()
    }

    fn set_trampoline(func: Self::PresentFn) {
        todo!()
    }

    fn new_present_fn() -> Self::PresentFn {
        todo!()
    }

    fn set_detour(detour: retour::RawDetour) {
        todo!()
    }
}

pub(super) fn dx12_duplicate_hook(
    this: &IDXGISwapChain,
    header: &SharedMemoryHeader,
) -> Result<(), windows::core::Error> {
    static SHARED_BUFFER: OnceLock<ID3D12Resource> = OnceLock::new();

    let device: ID3D12Device = unsafe { this.GetDevice() }?;
    header.set_api(shmem::RenderingAPI::Dx12);

    if let Some(buffer) = SHARED_BUFFER.get() {
        let command_allocator: ID3D12CommandAllocator =
            unsafe { device.CreateCommandAllocator(D3D12_COMMAND_LIST_TYPE_COPY) }.expect("WORK");

        let command_list: ID3D12GraphicsCommandList = unsafe {
            device
                .CreateCommandList(0, D3D12_COMMAND_LIST_TYPE_COPY, &command_allocator, None)
                .expect("WORK")
        };
        let resourec: ID3D12Resource = unsafe { this.GetBuffer(0) }.expect("WORK");

        unsafe { command_list.CopyResource(buffer, &resourec) };
    } else {
        let handle = header.get_nt_shared_handle();
        if let Some(handle) = handle {
            let mut resource = None;
            unsafe { device.OpenSharedHandle(handle, &mut resource) }.expect("WORK");

            if let Some(resource) = resource {
                SHARED_BUFFER.get_or_init(|| resource);
            }
        }
    }

    Ok(())
}
