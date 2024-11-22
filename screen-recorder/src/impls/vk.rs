use crate::impls::dxgi_impls::WAS_OPENGL_CALL;
use crate::{RenderingAPI, SHARED_CPU_BUFFER};
use retour::RawDetour;
use std::collections::HashSet;
use std::ffi::{c_char, CString};
use std::num::NonZero;
use std::ptr::{null, null_mut};
use std::sync::atomic::{AtomicU32, AtomicUsize, Ordering};
use std::sync::OnceLock;
use vulkanalia::loader::{LibloadingLoader, Loader, LIBRARY};
use vulkanalia::vk::{
    CommandBuffer, DeviceCommands, DeviceCreateInfo, DeviceMemory, DeviceV1_0, EntryV1_0,
    EntryV1_1, ExternalMemoryHandleTypeFlags, Fence, Handle, HasBuilder, Image, ImageLayout,
    ImportMemoryWin32HandleInfoKHR, InstanceCreateInfo, InstanceV1_0, MemoryAllocateInfo,
    PFN_vkAcquireNextImageKHR, PFN_vkCreateDevice, PFN_vkCreateInstance, PFN_vkCreateSwapchainKHR,
    PFN_vkEnumeratePhysicalDevices, PFN_vkGetDeviceProcAddr, PFN_vkGetInstanceProcAddr,
    PFN_vkQueuePresentKHR, PhysicalDevice, PresentInfoKHR, Queue, Result as VkResult, Semaphore,
    StructureType, SwapchainKHR,
};
use vulkanalia::{Device, Entry, Instance};
use windows::core::{s, PCSTR};
use windows::Win32::Foundation::HMODULE;
use windows::Win32::System::LibraryLoader::{GetModuleFileNameA, GetProcAddress};

static VK_PRESENT: OnceLock<<VkHooks as RenderingAPI>::PresentFn> = OnceLock::new();
static DETOUR: OnceLock<RawDetour> = OnceLock::new();
static NEXT_IMAGE_DETOUR: OnceLock<RawDetour> = OnceLock::new();

pub struct VkHooks {
    device: vulkanalia::vk::Device,
    device_commands: vulkanalia::vk::DeviceCommands,
    instance: vulkanalia::vk::Instance,
}

impl RenderingAPI for VkHooks {
    type PresentFn = PFN_vkQueuePresentKHR;

    type ResizeFn = PFN_vkCreateSwapchainKHR;

    fn present_fn(&self) -> *const () {
        self.device_commands.queue_present_khr as _
    }

    fn resize_fn(&self) -> *const () {
        self.device_commands.create_swapchain_khr as _
    }

    fn create(module: HMODULE) -> Result<Self, crate::error::Error> {
        const INSTANCE_EXTENSIONS: &[*const c_char] = &[
            c"VK_KHR_external_memory_capabilities".as_ptr(),
            c"VK_KHR_get_physical_device_properties2".as_ptr(),
        ];
        const DEVICE_EXTENSIONS: &[*const c_char] = &[
            c"VK_KHR_swapchain".as_ptr(),
            c"VK_KHR_external_memory".as_ptr(),
            c"VK_KHR_external_memory_win32".as_ptr(),
        ];

        let vk_get_instance_proc_addr =
            unsafe { GetProcAddress(module, s!("vkGetInstanceProcAddr")) }.unwrap();
        let vk_get_instance_proc_addr: PFN_vkGetInstanceProcAddr =
            unsafe { std::mem::transmute(vk_get_instance_proc_addr) };

        let vk_get_device_proc_addr =
            unsafe { GetProcAddress(module, s!("vkGetDeviceProcAddr")) }.unwrap();
        let vk_get_device_proc_addr: PFN_vkGetDeviceProcAddr =
            unsafe { std::mem::transmute(vk_get_device_proc_addr) };

        let create_instance = unsafe {
            vk_get_instance_proc_addr(
                vulkanalia::vk::Instance::null(),
                c"vkCreateInstance".as_ptr(),
            )
        }
        .unwrap();
        let vk_create_instance: PFN_vkCreateInstance =
            unsafe { std::mem::transmute(create_instance) };

        let application_info = vulkanalia::vk::ApplicationInfo::builder()
            .application_name(b"Vulkan Tutorial\0")
            .application_version(vulkanalia::vk::make_version(1, 0, 0))
            .engine_name(b"No Engine\0")
            .engine_version(vulkanalia::vk::make_version(1, 0, 0))
            .api_version(vulkanalia::vk::make_version(1, 0, 0));

        let info = InstanceCreateInfo::builder()
            .application_info(&application_info)
            .enabled_extension_names(&[]);

        println!("Attempting to create Instance");

        let mut instance = vulkanalia::vk::Instance::default();
        let result = unsafe { vk_create_instance(&*info, null(), &mut instance) };

        println!("The function at least ran!");

        if result != VkResult::SUCCESS {
            panic!("Fuck");
        }

        println!("Holy shit balls");

        let enumerate_physical_devices =
            unsafe { vk_get_instance_proc_addr(instance, c"vkEnumeratePhysicalDevices".as_ptr()) }
                .unwrap();

        let enumerate_physical_devices: PFN_vkEnumeratePhysicalDevices =
            unsafe { std::mem::transmute(enumerate_physical_devices) };

        let mut count = 0;
        let result = unsafe { enumerate_physical_devices(instance, &mut count, null_mut()) };

        if result != VkResult::SUCCESS {
            panic!("Fuck1");
        }

        println!("{count:?}");
        let mut physical_devices =
            vec![PhysicalDevice::default(); count as usize].into_boxed_slice();
        let result = unsafe {
            enumerate_physical_devices(instance, &mut count, physical_devices.as_mut_ptr())
        };

        if result != VkResult::SUCCESS {
            panic!("Fuck2");
        }

        let info = DeviceCreateInfo::builder().enabled_extension_names(DEVICE_EXTENSIONS);

        let physical_device = physical_devices[0];

        let vk_create_device =
            unsafe { vk_get_instance_proc_addr(instance, c"vkCreateDevice".as_ptr()) }.unwrap();

        let vk_create_device: PFN_vkCreateDevice = unsafe { std::mem::transmute(vk_create_device) };

        let mut device = vulkanalia::vk::Device::default();
        let result = unsafe { vk_create_device(physical_device, &*info, null(), &mut device) };

        if result != VkResult::SUCCESS {
            panic!("Fuck4")
        }

        let device_commands = unsafe {
            DeviceCommands::load(
                |name| vk_get_instance_proc_addr(instance, name),
                |name| vk_get_device_proc_addr(device, name),
            )
        };

        COMMANDS.get_or_init(|| device_commands);

        let acquire_next_image_khr =
            unsafe { vk_get_device_proc_addr(device, c"vkAcquireNextImageKHR".as_ptr()) }.unwrap();

        let detour =
            unsafe { RawDetour::new(acquire_next_image_khr as _, vk_new_acquire_next_image as _)? };

        unsafe {
            NEXT_IMAGE_DETOUR.get_or_init(|| detour).enable()?;
        }

        Ok(Self {
            device,
            instance,
            device_commands,
        })
    }

    fn destroy(&self) -> Result<(), crate::error::Error> {
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
static NEXT_IMAGE: AtomicU32 = AtomicU32::new(0);

unsafe extern "system" fn vk_new_acquire_next_image(
    device: vulkanalia::vk::Device,
    swapchain: SwapchainKHR,
    timeout: u64,
    semaphore: Semaphore,
    fence: Fence,
    image_index: *mut u32,
) -> VkResult {
    println!("A");

    if DEVICE.load(Ordering::SeqCst) == 0 {
        DEVICE.store(device.as_raw(), Ordering::SeqCst);
    }

    let acquire_next: PFN_vkAcquireNextImageKHR =
        unsafe { std::mem::transmute(NEXT_IMAGE_DETOUR.get().unwrap().trampoline()) };

    let result = unsafe { acquire_next(device, swapchain, timeout, semaphore, fence, image_index) };
    NEXT_IMAGE.store(*image_index, Ordering::SeqCst);

    result
}

static COMMANDS: OnceLock<DeviceCommands> = OnceLock::new();

unsafe extern "system" fn vk_new_queue_present(
    queue: Queue,
    info: *const PresentInfoKHR,
) -> VkResult {
    WAS_OPENGL_CALL.store(true, Ordering::SeqCst);

    let device = vulkanalia::vk::Device::from_raw(DEVICE.load(Ordering::SeqCst));
    let commands = COMMANDS.get().unwrap();

    let info = unsafe { &*info };
    let swapchain = unsafe { &*info.swapchains };

    if !device.is_null() {
        let header = SHARED_CPU_BUFFER.get().unwrap().0.header();
        header.set_api(shmem::RenderingAPI::Vk);
        header.set_width_and_height(1920, 1080);
        if let Some(nt_handle) = header.get_nt_shared_handle() {
            let mut shared_image = Image::null();

            // TODO: Need a real image info struct
            let result = (commands.create_image)(device, null(), null(), &mut shared_image);

            if result == VkResult::SUCCESS {
                let result = (commands.allocate_memory)(device, null(), null(), null_mut());

                if result != VkResult::SUCCESS {
                    println!("attempt 2: {result}")
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

                let result =
                    unsafe { (commands.allocate_memory)(device, &info, null(), &mut memory) };

                if result != VkResult::SUCCESS {
                    println!("{result}")
                }

                (commands.bind_image_memory)(device, shared_image, memory, 0);

                let mut command_buffer = CommandBuffer::null();
                (commands.allocate_command_buffers)(device, null(), &mut command_buffer); // TODO: Use proper info

                (commands.begin_command_buffer)(command_buffer, null()); // TODO: Use proper info

                let mut len = 0;
                let mut images = Vec::new();
                let result =
                    (commands.get_swapchain_images_khr)(device, *swapchain, &mut len, null_mut());
                if result != VkResult::SUCCESS {
                    println!("{result}")
                }
                images.reserve(len as usize);

                let result = (commands.get_swapchain_images_khr)(
                    device,
                    *swapchain,
                    &mut len,
                    images.as_mut_ptr(),
                );

                if result != VkResult::SUCCESS {
                    println!("{result}")
                }

                (commands.cmd_copy_image)(
                    command_buffer,
                    images[NEXT_IMAGE.load(Ordering::SeqCst) as usize],
                    ImageLayout::default(),
                    shared_image,
                    ImageLayout::default(),
                    0,
                    null(),
                );

                (commands.cmd_execute_commands)(command_buffer, 0, null());

                let result = (commands.end_command_buffer)(command_buffer);

                if result != VkResult::SUCCESS {
                    println!("{result}")
                }
            } else {
                println!("{result}")
            }
        }
    }

    println!("Welp");

    VK_PRESENT.get().unwrap()(queue, info)
}
