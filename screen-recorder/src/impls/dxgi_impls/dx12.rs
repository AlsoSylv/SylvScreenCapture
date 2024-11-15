use std::{mem::ManuallyDrop, sync::OnceLock};

use retour::RawDetour;
use shmem::SharedMemoryHeader;
use windows::{
    core::{s, Interface},
    Win32::{
        Foundation::HWND,
        Graphics::{
            Direct3D::{D3D_FEATURE_LEVEL, D3D_FEATURE_LEVEL_12_0},
            Direct3D12::{
                ID3D12CommandAllocator, ID3D12CommandQueue, ID3D12Device, ID3D12Fence,
                ID3D12GraphicsCommandList, ID3D12Resource, D3D12_COMMAND_LIST_TYPE_COPY,
                D3D12_COMMAND_QUEUE_DESC, D3D12_FENCE_FLAG_NONE, D3D12_TEXTURE_COPY_LOCATION,
                D3D12_TEXTURE_COPY_LOCATION_0, D3D12_TEXTURE_COPY_TYPE_SUBRESOURCE_INDEX,
            },
            Dxgi::{CreateDXGIFactory, IDXGIFactory, IDXGISwapChain},
        },
        System::{
            LibraryLoader::GetProcAddress,
            Threading::{CreateEventA, WaitForSingleObject, INFINITE},
        },
        UI::WindowsAndMessaging::WNDCLASSEXA,
    },
};

use crate::RenderingAPI;

static DETOUR: OnceLock<RawDetour> = OnceLock::new();

// static EVENT_QUEUE_DETOUR: OnceLock<RawDetour> = OnceLock::new();
// static EVENT_QUEUE_TRAMPOLINE: OnceLock<
//     unsafe extern "system" fn(*mut c_void, u32, *const *mut c_void),
// > = OnceLock::new();

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
            D3D12CreateDevice(
                None,
                D3D_FEATURE_LEVEL_12_0,
                &ID3D12Device::IID,
                &mut device,
            )
        };

        if result.is_err() {
            println!("FUCK");
            Err(windows::core::Error::from_hresult(result))?;
        }

        let device = device.unwrap();

        let description = D3D12_COMMAND_QUEUE_DESC::default();
        let command_queue: ID3D12CommandQueue = unsafe { device.CreateCommandQueue(&description) }?;
        // let execute_command_list_ptr = command_queue.vtable().ExecuteCommandLists;
        // let detour = unsafe {
        //     RawDetour::new(
        //         execute_command_list_ptr as _,
        //         execute_command_list_hook as _,
        //     )
        // }?;
        // unsafe { detour.enable() };
        // EVENT_QUEUE_TRAMPOLINE.get_or_init(|| unsafe { std::mem::transmute(detour.trampoline()) });
        // EVENT_QUEUE_DETOUR.get_or_init(|| detour);

        let (window, window_class) = unsafe { super::super::create_window() }?;

        let factory: IDXGIFactory = unsafe { CreateDXGIFactory() }?;

        let description = super::dxgi_swap_chain_desc(window);

        let mut swap_chain = None;

        let result =
            unsafe { factory.CreateSwapChain(&command_queue, &description, &mut swap_chain) };

        if result.is_err() {
            Err(windows::core::Error::from_hresult(result))?;
        }

        let swap_chain = swap_chain.unwrap();

        Ok(Self {
            window,
            window_class,
            swap_chain,
        })
    }

    fn destroy(&self) -> Result<(), crate::error::Error> {
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

// static QUEUE: AtomicPtr<c_void> = AtomicPtr::new(null_mut());
//
// unsafe extern "system" fn execute_command_list_hook(
//     this: *mut c_void,
//     count: u32,
//     command_list: *const *mut c_void,
// ) {
//     static ONCE: Once = Once::new();
//     ONCE.call_once(|| {
//         QUEUE.store(this, std::sync::atomic::Ordering::SeqCst);
//     });
//     let trampoline = EVENT_QUEUE_TRAMPOLINE.get().expect("This was set earlier");
//
//     unsafe { trampoline(this, count, command_list) }
// }

pub(super) fn dx12_duplicate_hook(
    this: &IDXGISwapChain,
    header: &SharedMemoryHeader,
) -> Result<(), windows::core::Error> {
    static SHARED_BUFFER: OnceLock<ID3D12Resource> = OnceLock::new();
    static QUEUE: OnceLock<(
        ID3D12CommandQueue,
        ID3D12CommandAllocator,
        ID3D12GraphicsCommandList,
    )> = OnceLock::new();

    let device: ID3D12Device = unsafe { this.GetDevice() }?;
    header.set_api(shmem::RenderingAPI::Dx12);
    header.set_width_and_height(1920, 1080);

    if let Some(buffer) = SHARED_BUFFER.get() {
        if let Some((queue, command_allocator, command_list)) = QUEUE.get() {
            let back_buffer: ID3D12Resource = unsafe { this.GetBuffer(0)? };

            let dst_loc = texture_copy_location(buffer);
            let src_loc = texture_copy_location(&back_buffer);

            unsafe {
                command_list.CopyTextureRegion(&dst_loc, 0, 0, 0, &src_loc, None);
            }
            unsafe {
                if let Err(e) = command_list.Close() {
                    println!("Terminating early, command list is invalid: {e:?}");
                    return Ok(());
                };
            }
            unsafe {
                queue.ExecuteCommandLists(&[Some(command_list.clone().into())]);
            }
            let fence: ID3D12Fence = unsafe { device.CreateFence(0, D3D12_FENCE_FLAG_NONE) }?;
            let auto_reset = unsafe { CreateEventA(None, false, None, None) }?;
            unsafe { fence.SetEventOnCompletion(0, auto_reset) }?;
            unsafe { queue.Wait(&fence, 0)? };
            unsafe { WaitForSingleObject(auto_reset, INFINITE) };

            unsafe { command_allocator.Reset()? };
            unsafe { command_list.Reset(command_allocator, None)? };
        } else {
            let queue = unsafe {
                device.CreateCommandQueue(&D3D12_COMMAND_QUEUE_DESC {
                    Type: D3D12_COMMAND_LIST_TYPE_COPY,
                    ..Default::default()
                })
            }?;

            let command_allocator =
                unsafe { device.CreateCommandAllocator(D3D12_COMMAND_LIST_TYPE_COPY) }?;

            let command_list = unsafe {
                device.CreateCommandList(
                    0,
                    D3D12_COMMAND_LIST_TYPE_COPY,
                    &command_allocator,
                    None,
                )?
            };

            QUEUE.get_or_init(|| (queue, command_allocator, command_list));
        }
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

fn texture_copy_location(texture: &ID3D12Resource) -> D3D12_TEXTURE_COPY_LOCATION {
    D3D12_TEXTURE_COPY_LOCATION {
        Type: D3D12_TEXTURE_COPY_TYPE_SUBRESOURCE_INDEX,
        pResource: ManuallyDrop::new(Some(texture.clone())),
        Anonymous: D3D12_TEXTURE_COPY_LOCATION_0 {
            SubresourceIndex: 0,
        },
    }
}
