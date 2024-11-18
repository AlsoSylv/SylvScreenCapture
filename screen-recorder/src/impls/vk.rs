use crate::{RenderingAPI, SHARED_CPU_BUFFER};
use retour::RawDetour;
use std::ffi::c_char;
use std::ptr::{null, null_mut};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::OnceLock;
use vulkanalia::loader::LibloadingLoader;
use vulkanalia::vk::{BufferCreateFlags, BufferUsageFlags, DeviceCommands, DeviceCreateInfo, DeviceMemory, DeviceV1_0, ExternalMemoryHandleTypeFlags, Fence, Handle, HasBuilder, Image, ImportMemoryWin32HandleInfoKHR, InstanceCreateInfo, InstanceV1_0, MemoryAllocateInfo, MemoryGetWin32HandleInfoKHR, PFN_vkAcquireNextImageKHR, PFN_vkCreateSwapchainKHR, PFN_vkQueuePresentKHR, PhysicalDeviceExternalBufferInfo, PhysicalDeviceExternalBufferInfoKHR, PresentInfoKHR, Queue, Result as VkResult, Semaphore, StructureType, SwapchainKHR};
use vulkanalia::{Device, Entry, Instance};
use windows::Win32::Foundation::{HANDLE, HMODULE};
use windows::Win32::System::LibraryLoader::GetModuleFileNameA;

static VK_PRESENT: OnceLock<<VkHooks as RenderingAPI>::PresentFn> = OnceLock::new();
static DETOUR: OnceLock<RawDetour> = OnceLock::new();
static NEXT_IMAGE_DETOUR: OnceLock<RawDetour> = OnceLock::new();

pub struct VkHooks {
    device: Device,
    instance: Instance,
}

impl RenderingAPI for VkHooks {
    type PresentFn = PFN_vkQueuePresentKHR;

    type ResizeFn = PFN_vkCreateSwapchainKHR;

    fn present_fn(&self) -> *const () {
        self.device.commands().queue_present_khr as _
    }

    fn resize_fn(&self) -> *const () {
        self.device.commands().create_swapchain_khr as _
    }

    fn create(module: HMODULE) -> Result<Self, crate::error::Error> {
        let mut buffer = [0; 128];

        let res = unsafe { GetModuleFileNameA(module, &mut buffer) };

        if res == 0 {
            Err(windows::core::Error::from_win32())?
        }

        let (idx, _) = buffer
            .iter()
            .enumerate()
            .rfind(|(_, byte)| **byte == 0)
            .unwrap();

        let name = std::str::from_utf8(&buffer[0..idx]).unwrap();

        let loader = unsafe { LibloadingLoader::new(name).unwrap() };

        let entry = unsafe { Entry::new(loader).unwrap() };

        const INSTANCE_EXTENSIONS: &[*const c_char] = &[
            c"VK_KHR_external_memory_capabilities".as_ptr(),
            c"VK_KHR_get_physical_device_properties2".as_ptr(),
        ];
        const DEVICE_EXTENSIONS: &[*const c_char] = &[
            c"VK_KHR_swapchain".as_ptr(),
            c"VK_KHR_external_memory".as_ptr(),
            c"VK_KHR_external_memory_win32".as_ptr(),
        ];

        let info = InstanceCreateInfo::builder().enabled_extension_names(INSTANCE_EXTENSIONS);

        let instance = unsafe { entry.create_instance(&info, None).unwrap() };

        let physical_device = unsafe { instance.enumerate_physical_devices() }.unwrap()[0];

        let info = DeviceCreateInfo::builder().enabled_extension_names(DEVICE_EXTENSIONS);

        let device = unsafe {
            instance
                .create_device(physical_device, &info, None)
                .unwrap()
        };

        COMMANDS.get_or_init(|| device.commands().clone());

        let command = device.commands().acquire_next_image_khr;

        let detour = unsafe { RawDetour::new(command as _, vk_new_acquire_next_image as _)? };

        unsafe {
            DETOUR.get_or_init(|| detour).enable()?;
        }

        // TODO: Hook AcquireNextImage

        Ok(Self { device, instance })
    }

    fn destroy(&self) -> Result<(), crate::error::Error> {
        unsafe { self.device.destroy_device(None) }
        unsafe { self.instance.destroy_instance(None) }

        Ok(())
    }

    fn trampoline() -> Self::PresentFn {
        *VK_PRESENT.get().unwrap()
    }

    fn set_trampoline(func: Self::PresentFn) {
        VK_PRESENT.set(func).unwrap()
    }

    fn new_present_fn() -> Self::PresentFn {
        vk_new_queue_present
    }

    fn set_detour(detour: RawDetour) {
        DETOUR.set(detour).unwrap()
    }
}

static DEVICE: AtomicUsize = AtomicUsize::new(0);

unsafe extern "system" fn vk_new_acquire_next_image(
    device: vulkanalia::vk::Device,
    swapchain: SwapchainKHR,
    timeout: u64,
    semaphore: Semaphore,
    fence: Fence,
    image_index: *mut u32,
) -> VkResult {
    if DEVICE.load(Ordering::SeqCst) == 0 {
        DEVICE.store(device.as_raw(), Ordering::SeqCst);
    }

    let acquire_next: PFN_vkAcquireNextImageKHR =
        unsafe { std::mem::transmute(NEXT_IMAGE_DETOUR.get().unwrap().trampoline()) };

    unsafe { acquire_next(device, swapchain, timeout, semaphore, fence, image_index) }
}

static COMMANDS: OnceLock<DeviceCommands> = OnceLock::new();

unsafe extern "system" fn vk_new_queue_present(
    queue: Queue,
    info: *const PresentInfoKHR,
) -> VkResult {
    let device = vulkanalia::vk::Device::from_raw(DEVICE.load(Ordering::SeqCst));
    let commands = COMMANDS.get().unwrap();

    if !device.is_null() {
        let header = SHARED_CPU_BUFFER.get().unwrap().0.header();
        if let Some(nt_handle) = header.get_nt_shared_handle() {
            let result = (commands.allocate_memory)(device, null(), null(), null_mut());

            if result != VkResult::SUCCESS {
                println!("{result}")
            }
                
            let info = ImportMemoryWin32HandleInfoKHR {
                s_type: StructureType::IMPORT_MEMORY_WIN32_HANDLE_INFO_KHR,
                next: null(),
                handle_type: ExternalMemoryHandleTypeFlags::D3D11_TEXTURE,
                handle: nt_handle.0,
                name: null(),
            };

            let info = MemoryAllocateInfo {
                allocation_size: 0,
                next: &info as *const _ as _,
                memory_type_index: 0,
                s_type: StructureType::MEMORY_ALLOCATE_INFO,
            };

            let mut memory = DeviceMemory::null();

            let result = unsafe { (commands.allocate_memory)(device, &info, null(), &mut memory) };
    
            if result != VkResult::SUCCESS {
                println!("{result}")
            }

            let image = Image::null(); // TODO: Make a real vulkan image

            (commands.bind_image_memory)(device, image, memory, 0);
    
            // TODO: copy buffer
        }


    }

    VK_PRESENT.get().unwrap()(queue, info)
}
