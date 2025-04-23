use crate::impls::dxgi_impls::WAS_OPENGL_CALL;
use crate::{RenderingAPI, SHARED_CPU_BUFFER};
use retour::RawDetour;
use std::ffi::c_char;
use std::ptr::{null, null_mut};
use std::sync::atomic::{AtomicPtr, AtomicU32, AtomicUsize, Ordering};
use std::sync::OnceLock;
use vulkanalia::vk::{
    self, CommandBuffer, CommandBufferAllocateInfo, CommandBufferBeginInfo, CommandBufferLevel,
    CommandBufferUsageFlags, CommandPool, CommandPoolCreateFlags, CommandPoolCreateInfo,
    DependencyFlags, DeviceCommands, DeviceCreateInfo, DeviceMemory, Extent3D,
    ExternalMemoryHandleTypeFlags, ExternalMemoryImageCreateInfo, Fence, Format, Handle,
    HasBuilder, Image, ImageAspectFlags, ImageCopy, ImageCreateInfo, ImageLayout,
    ImageSubresourceLayers, ImageType, ImageUsageFlags, ImportMemoryWin32HandleInfoKHR,
    InstanceCommands, InstanceCreateInfo, MemoryAllocateInfo, MemoryDedicatedAllocateInfoKHR,
    PFN_vkAcquireNextImageKHR, PFN_vkCreateDevice, PFN_vkCreateInstance, PFN_vkCreateSwapchainKHR,
    PFN_vkEnumeratePhysicalDevices, PFN_vkGetDeviceProcAddr, PFN_vkGetInstanceProcAddr,
    PFN_vkGetSwapchainImagesKHR, PFN_vkQueuePresentKHR, PhysicalDevice, PipelineStageFlags,
    PresentInfoKHR, Queue, Result as VkResult, SampleCountFlags, Semaphore, SharingMode,
    SwapchainKHR,
};
use windows::core::s;
use windows::Win32::Foundation::HMODULE;
use windows::Win32::System::LibraryLoader::GetProcAddress;

static VK_PRESENT: OnceLock<<VkHooks as RenderingAPI>::PresentFn> = OnceLock::new();
static DETOUR: OnceLock<RawDetour> = OnceLock::new();
static NEXT_IMAGE_DETOUR: OnceLock<RawDetour> = OnceLock::new();

pub struct VkHooks {
    device: vulkanalia::vk::Device,
    device_commands: vulkanalia::vk::DeviceCommands,
    instance: vulkanalia::vk::Instance,
    instance_commands: vulkanalia::vk::InstanceCommands,
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
            .enabled_extension_names(INSTANCE_EXTENSIONS)
            .application_info(&application_info)
            .enabled_extension_names(&[]);

        println!("Attempting to create Instance");

        let mut instance = vulkanalia::vk::Instance::default();
        let result = unsafe { vk_create_instance(&*info, null(), &mut instance) };

        println!("The function at least ran!");

        if result != VkResult::SUCCESS {
            panic!("Fuck");
        }

        let instance_commands =
            unsafe { InstanceCommands::load(|ptr| vk_get_instance_proc_addr(instance, ptr)) };

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

        let mut mem_ty_idx = 0;
        let prop_flag = vk::MemoryPropertyFlags::DEVICE_LOCAL;

        let mut props = vk::PhysicalDeviceMemoryProperties::default();
        unsafe {
            (instance_commands.get_physical_device_memory_properties)(physical_device, &mut props)
        };

        for i in 0..props.memory_type_count {
            if (1 << i) & prop_flag.bits() != 0 {
                mem_ty_idx = i;
            }
        }

        MEM_TY_IDX.store(mem_ty_idx, Ordering::SeqCst);

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

        let get_swapchain_images =
            unsafe { vk_get_device_proc_addr(device, c"vkGetSwapchainImagesKHR".as_ptr()) }
                .unwrap();

        let detour =
            unsafe { RawDetour::new(get_swapchain_images as _, vk_new_get_swapchain_images as _)? };

        unsafe {
            GET_SWAPCHAIN_IMAGES.get_or_init(|| detour).enable()?;
        }

        let instance_commands =
            unsafe { InstanceCommands::load(|name| vk_get_instance_proc_addr(instance, name)) };

        Ok(Self {
            device,
            instance,
            device_commands,
            instance_commands,
        })
    }

    fn destroy(&self) -> Result<(), crate::error::Error> {
        unsafe { (self.device_commands.destroy_device)(self.device, null()) };
        unsafe { (self.instance_commands.destroy_instance)(self.instance, null()) }
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

static MEM_TY_IDX: AtomicU32 = AtomicU32::new(0);
static DEVICE: AtomicUsize = AtomicUsize::new(0);
static IMAGES: AtomicPtr<Image> = AtomicPtr::new(null_mut());

static GET_SWAPCHAIN_IMAGES: OnceLock<RawDetour> = OnceLock::new();

unsafe extern "system" fn vk_new_get_swapchain_images(
    device: vk::Device,
    swapchain: SwapchainKHR,
    swapchain_image_count: *mut u32,
    swapchain_images: *mut Image,
) -> VkResult {
    IMAGES.store(swapchain_images, Ordering::SeqCst);

    let get_images: PFN_vkGetSwapchainImagesKHR =
        unsafe { std::mem::transmute(GET_SWAPCHAIN_IMAGES.get().unwrap().trampoline()) };

    get_images(device, swapchain, swapchain_image_count, swapchain_images)
}

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
    present_info: *const PresentInfoKHR,
) -> VkResult {
    static SHARED_VK_BUFFER: OnceLock<vk::Image> = OnceLock::new();
    static COMMAND_POOL: OnceLock<CommandPool> = OnceLock::new();
    static IMAGES: OnceLock<Box<[vk::Image]>> = OnceLock::new();

    WAS_OPENGL_CALL.store(true, Ordering::SeqCst);

    let device = vulkanalia::vk::Device::from_raw(DEVICE.load(Ordering::SeqCst));
    let commands = COMMANDS.get().unwrap();

    let info = unsafe { &*present_info };
    let idx = unsafe { *info.image_indices };
    let swapchain = unsafe { &*info.swapchains };

    if !device.is_null() {
        let header = SHARED_CPU_BUFFER.read().unwrap();
        header.set_api(shmem::RenderingAPI::Vk);
        header.set_width_and_height(1920, 1080);
        if let (Some(shared_image), Some(command_pool)) =
            (SHARED_VK_BUFFER.get(), COMMAND_POOL.get())
        {
            let result = (commands.queue_wait_idle)(queue);

            println!("await queue: {result}");

            let mut command_buffer = CommandBuffer::null();

            let info = CommandBufferAllocateInfo::builder()
                .command_buffer_count(1)
                .command_pool(*command_pool)
                .level(CommandBufferLevel::PRIMARY);

            (commands.allocate_command_buffers)(device, &*info, &mut command_buffer);
            let info =
                CommandBufferBeginInfo::builder().flags(CommandBufferUsageFlags::ONE_TIME_SUBMIT);

            (commands.begin_command_buffer)(command_buffer, &*info);

            let images = &**IMAGES.get_or_init(|| {
                let mut len = 0;

                let res =
                    (commands.get_swapchain_images_khr)(device, *swapchain, &mut len, null_mut());

                println!("get_len: {res}");

                let mut slice = vec![Image::default(); len as usize];

                let res = (commands.get_swapchain_images_khr)(
                    device,
                    *swapchain,
                    &mut len,
                    slice.as_mut_ptr(),
                );

                println!("get_images: {res}");

                slice.into_boxed_slice()
            });

            let image = images[idx as usize];

            let memory_barrier = vk::ImageMemoryBarrier::builder()
                .old_layout(ImageLayout::PRESENT_SRC_KHR)
                .new_layout(ImageLayout::TRANSFER_SRC_OPTIMAL)
                .src_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
                .dst_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
                .src_access_mask(vk::AccessFlags::MEMORY_READ)
                .dst_access_mask(vk::AccessFlags::TRANSFER_READ)
                .subresource_range(vk::ImageSubresourceRange {
                    aspect_mask: vk::ImageAspectFlags::COLOR,
                    base_array_layer: 0,
                    level_count: 1,
                    base_mip_level: 0,
                    layer_count: 1,
                })
                .image(image);

            (commands.cmd_pipeline_barrier)(
                command_buffer,
                PipelineStageFlags::TRANSFER,
                PipelineStageFlags::TRANSFER,
                DependencyFlags::empty(),
                0,
                null(),
                0,
                null(),
                1,
                &*memory_barrier,
            );

            let image_copy = ImageCopy::builder()
                .dst_subresource(ImageSubresourceLayers {
                    base_array_layer: 0,
                    aspect_mask: ImageAspectFlags::COLOR,
                    mip_level: 1,
                    layer_count: 1,
                })
                .src_subresource(ImageSubresourceLayers {
                    base_array_layer: 0,
                    aspect_mask: ImageAspectFlags::COLOR,
                    mip_level: 1,
                    layer_count: 1,
                });

            (commands.cmd_copy_image)(
                command_buffer,
                image,
                ImageLayout::TRANSFER_SRC_OPTIMAL,
                *shared_image,
                ImageLayout::TRANSFER_DST_OPTIMAL,
                1,
                &*image_copy,
            );

            (commands.cmd_execute_commands)(command_buffer, 0, null());

            let memory_barrier = vk::ImageMemoryBarrier::builder()
                .old_layout(ImageLayout::TRANSFER_SRC_OPTIMAL)
                .new_layout(ImageLayout::PRESENT_SRC_KHR)
                .src_access_mask(vk::AccessFlags::TRANSFER_READ)
                .dst_access_mask(vk::AccessFlags::MEMORY_READ)
                .src_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
                .dst_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
                .subresource_range(vk::ImageSubresourceRange {
                    aspect_mask: vk::ImageAspectFlags::COLOR,
                    base_array_layer: 0,
                    level_count: 1,
                    base_mip_level: 0,
                    layer_count: 1,
                })
                .image(image);

            (commands.cmd_pipeline_barrier)(
                command_buffer,
                PipelineStageFlags::TRANSFER,
                PipelineStageFlags::DRAW_INDIRECT,
                DependencyFlags::DEVICE_GROUP,
                0,
                null(),
                0,
                null(),
                1,
                &*memory_barrier,
            );

            let result = (commands.end_command_buffer)(command_buffer);

            println!("{result}");

            let buffers = &[command_buffer];

            let submit_info = vk::SubmitInfo::builder().command_buffers(buffers);

            let mut fence = vk::Fence::null();

            let result = unsafe {
                (commands.create_fence)(device, &vk::FenceCreateInfo::default(), null(), &mut fence)
            };

            println!("Create fence: {result}");

            let result = (commands.queue_submit)(queue, 1, &*submit_info, fence);

            println!("{result}");

            let result =
                unsafe { (commands.wait_for_fences)(device, 1, &fence, vk::TRUE, u64::MAX) };

            println!("{result}");

            (commands.free_command_buffers)(device, *command_pool, 1, &command_buffer);
        } else if let Some(nt_handle) = header.get_nt_shared_handle() {
            let mut info = ExternalMemoryImageCreateInfo::builder()
                .handle_types(ExternalMemoryHandleTypeFlags::D3D11_TEXTURE);

            let info = ImageCreateInfo::builder()
                .image_type(ImageType::_2D)
                .array_layers(1)
                .mip_levels(1)
                .samples(SampleCountFlags::_1)
                .format(Format::R8G8B8A8_SNORM)
                .initial_layout(ImageLayout::TRANSFER_DST_OPTIMAL)
                .sharing_mode(SharingMode::EXCLUSIVE)
                .push_next(&mut info)
                .usage(ImageUsageFlags::TRANSFER_DST | ImageUsageFlags::COLOR_ATTACHMENT)
                .extent(Extent3D {
                    width: 1920,
                    height: 1080,
                    depth: 1,
                });
            let mut shared_image = Image::null();

            // TODO: Need a real image info struct
            let result = (commands.create_image)(device, &*info, null(), &mut shared_image);

            if result == VkResult::SUCCESS {
                let mut ded_info = MemoryDedicatedAllocateInfoKHR::builder().image(shared_image);

                let mut info = ImportMemoryWin32HandleInfoKHR::builder()
                    .handle_type(ExternalMemoryHandleTypeFlags::D3D11_TEXTURE)
                    .handle(nt_handle.0);

                let info = MemoryAllocateInfo::builder()
                    .allocation_size(1920 * 1080 * 4)
                    .memory_type_index(MEM_TY_IDX.load(Ordering::SeqCst))
                    .push_next(&mut info)
                    .push_next(&mut ded_info);

                let mut memory = DeviceMemory::null();

                let result =
                    unsafe { (commands.allocate_memory)(device, &*info, null(), &mut memory) };

                println!("{result}");

                let result = (commands.bind_image_memory)(device, shared_image, memory, 0);

                println!("{result}");

                let mut command_pool = CommandPool::null();

                let info =
                    CommandPoolCreateInfo::builder().flags(CommandPoolCreateFlags::TRANSIENT);
                let result =
                    (commands.create_command_pool)(device, &*info, null(), &mut command_pool);

                println!("{result}");

                if result == VkResult::SUCCESS {
                    COMMAND_POOL.get_or_init(|| command_pool);
                    SHARED_VK_BUFFER.get_or_init(|| shared_image);
                }
            }
        }
    }

    let result = VK_PRESENT.get().unwrap()(queue, info);

    result
}
