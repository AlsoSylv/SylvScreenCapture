pub use self::enumerations::*;
pub use self::functions::*;
pub use self::types::*;

use std::cell::Cell;
use std::os::raw::c_void;

#[derive(Clone)]
struct FnPtr {
    ptr: Cell<*const c_void>,
    is_loaded: Cell<bool>,
}

#[allow(dead_code)]
impl FnPtr {
    fn new(ptr: *const c_void) -> FnPtr {
        if !ptr.is_null() {
            FnPtr {
                ptr: Cell::new(ptr),
                is_loaded: Cell::new(true),
            }
        } else {
            FnPtr {
                ptr: Cell::new(FnPtr::not_initialized as *const c_void),
                is_loaded: Cell::new(false),
            }
        }
    }

    fn set_ptr(&self, ptr: *const c_void) {
        let new = Self::new(ptr);
        self.ptr.set(new.ptr.into_inner());
        self.is_loaded.set(new.is_loaded.into_inner());
    }

    fn aliased(&mut self, other: *const FnPtr) {
        unsafe {
            if !self.is_loaded.get() && (*other).is_loaded.get() {
                *self = (*other).clone();
            }
        }
    }

    #[inline(never)]
    fn not_initialized() -> ! {
        panic!("gl: function not initialized")
    }
}

unsafe impl Sync for FnPtr {}
unsafe impl Send for FnPtr {}

pub mod types {
    #![allow(dead_code, non_camel_case_types, non_snake_case)]

    use std;
    use std::os::raw::*;

    // types required for: xcb
    pub type xcb_connection_t = std::os::raw::c_void;
    pub type xcb_window_t = u32;
    pub type xcb_visualid_t = u32;
    // types required for: xlib(_xrandr)
    pub type Display = std::os::raw::c_void;
    pub type RROutput = std::os::raw::c_ulong;
    pub type Window = std::os::raw::c_ulong;
    pub type VisualID = std::os::raw::c_ulong;
    // types required for: win32
    pub type BOOL = std::os::raw::c_int;
    pub type DWORD = std::os::raw::c_ulong;
    pub type LPVOID = *mut std::os::raw::c_void;
    pub type HANDLE = *mut std::os::raw::c_void;
    pub type HMONITOR = *mut std::os::raw::c_void;
    pub type WCHAR = u16;
    pub type LPCWSTR = *const WCHAR;
    pub type HINSTANCE = *mut std::os::raw::c_void;
    pub type HWND = *mut std::os::raw::c_void;
    #[repr(C)]
    #[derive(Copy, Clone)]
    pub struct SECURITY_ATTRIBUTES {
        nLength: DWORD,
        lpSecurityDescriptor: LPVOID,
        bInheritHandle: BOOL,
    }
    // types required for: wayland
    pub type wl_display = std::os::raw::c_void;
    pub type wl_surface = std::os::raw::c_void;
    // types required for: mir
    pub type MirConnection = std::os::raw::c_void;
    pub type MirSurface = std::os::raw::c_void;

    #[macro_export]
    macro_rules! VK_MAKE_VERSION {
        ($major:expr, $minor:expr, $patch:expr) => {
            (($major) << 22) | (($minor) << 12) | ($patch)
        };
    }

    #[macro_export]
    macro_rules! VK_VERSION_MAJOR {
        ($version:expr) => {
            $version >> 22
        };
    }
    #[macro_export]
    macro_rules! VK_VERSION_MINOR {
        ($version:expr) => {
            ($version >> 12) & 0x3ff
        };
    }
    #[macro_export]
    macro_rules! VK_VERSION_PATCH {
        ($version:expr) => {
            $version & 0xfff
        };
    }

    #[macro_export]
    macro_rules! VK_DEFINE_NON_DISPATCHABLE_HANDLE {
        ($name:ident) => {
            #[repr(C)]
            #[derive(Copy, Clone)]
            pub struct $name(u64);
        };
    }

    #[macro_export]
    macro_rules! VK_DEFINE_HANDLE {
        ($name:ident) => {
            #[repr(C)]
            #[derive(Copy, Clone)]
            pub struct $name(*const std::os::raw::c_void);
        };
    }

    VK_DEFINE_HANDLE!(VkInstance);
    VK_DEFINE_HANDLE!(VkPhysicalDevice);
    VK_DEFINE_HANDLE!(VkDevice);
    VK_DEFINE_HANDLE!(VkQueue);
    VK_DEFINE_HANDLE!(VkCommandBuffer);
    VK_DEFINE_NON_DISPATCHABLE_HANDLE!(VkDeviceMemory);
    VK_DEFINE_NON_DISPATCHABLE_HANDLE!(VkCommandPool);
    VK_DEFINE_NON_DISPATCHABLE_HANDLE!(VkBuffer);
    VK_DEFINE_NON_DISPATCHABLE_HANDLE!(VkBufferView);
    VK_DEFINE_NON_DISPATCHABLE_HANDLE!(VkImage);
    VK_DEFINE_NON_DISPATCHABLE_HANDLE!(VkImageView);
    VK_DEFINE_NON_DISPATCHABLE_HANDLE!(VkShaderModule);
    VK_DEFINE_NON_DISPATCHABLE_HANDLE!(VkPipeline);
    VK_DEFINE_NON_DISPATCHABLE_HANDLE!(VkPipelineLayout);
    VK_DEFINE_NON_DISPATCHABLE_HANDLE!(VkSampler);
    VK_DEFINE_NON_DISPATCHABLE_HANDLE!(VkDescriptorSet);
    VK_DEFINE_NON_DISPATCHABLE_HANDLE!(VkDescriptorSetLayout);
    VK_DEFINE_NON_DISPATCHABLE_HANDLE!(VkDescriptorPool);
    VK_DEFINE_NON_DISPATCHABLE_HANDLE!(VkFence);
    VK_DEFINE_NON_DISPATCHABLE_HANDLE!(VkSemaphore);
    VK_DEFINE_NON_DISPATCHABLE_HANDLE!(VkEvent);
    VK_DEFINE_NON_DISPATCHABLE_HANDLE!(VkQueryPool);
    VK_DEFINE_NON_DISPATCHABLE_HANDLE!(VkFramebuffer);
    VK_DEFINE_NON_DISPATCHABLE_HANDLE!(VkRenderPass);
    VK_DEFINE_NON_DISPATCHABLE_HANDLE!(VkPipelineCache);

    #[repr(i32)]
    #[derive(Copy, Clone, Eq, PartialEq, Debug)]
    pub enum VkAttachmentLoadOp {
        VK_ATTACHMENT_LOAD_OP_LOAD = 0,
        VK_ATTACHMENT_LOAD_OP_CLEAR = 1,
        VK_ATTACHMENT_LOAD_OP_DONT_CARE = 2,
    }

    #[repr(i32)]
    #[derive(Copy, Clone, Eq, PartialEq, Debug)]
    pub enum VkAttachmentStoreOp {
        VK_ATTACHMENT_STORE_OP_STORE = 0,
        VK_ATTACHMENT_STORE_OP_DONT_CARE = 1,
    }

    #[repr(i32)]
    #[derive(Copy, Clone, Eq, PartialEq, Debug)]
    pub enum VkBlendFactor {
        VK_BLEND_FACTOR_ZERO = 0,
        VK_BLEND_FACTOR_ONE = 1,
        VK_BLEND_FACTOR_SRC_COLOR = 2,
        VK_BLEND_FACTOR_ONE_MINUS_SRC_COLOR = 3,
        VK_BLEND_FACTOR_DST_COLOR = 4,
        VK_BLEND_FACTOR_ONE_MINUS_DST_COLOR = 5,
        VK_BLEND_FACTOR_SRC_ALPHA = 6,
        VK_BLEND_FACTOR_ONE_MINUS_SRC_ALPHA = 7,
        VK_BLEND_FACTOR_DST_ALPHA = 8,
        VK_BLEND_FACTOR_ONE_MINUS_DST_ALPHA = 9,
        VK_BLEND_FACTOR_CONSTANT_COLOR = 10,
        VK_BLEND_FACTOR_ONE_MINUS_CONSTANT_COLOR = 11,
        VK_BLEND_FACTOR_CONSTANT_ALPHA = 12,
        VK_BLEND_FACTOR_ONE_MINUS_CONSTANT_ALPHA = 13,
        VK_BLEND_FACTOR_SRC_ALPHA_SATURATE = 14,
        VK_BLEND_FACTOR_SRC1_COLOR = 15,
        VK_BLEND_FACTOR_ONE_MINUS_SRC1_COLOR = 16,
        VK_BLEND_FACTOR_SRC1_ALPHA = 17,
        VK_BLEND_FACTOR_ONE_MINUS_SRC1_ALPHA = 18,
    }

    #[repr(i32)]
    #[derive(Copy, Clone, Eq, PartialEq, Debug)]
    pub enum VkBlendOp {
        VK_BLEND_OP_ADD = 0,
        VK_BLEND_OP_SUBTRACT = 1,
        VK_BLEND_OP_REVERSE_SUBTRACT = 2,
        VK_BLEND_OP_MIN = 3,
        VK_BLEND_OP_MAX = 4,
    }

    #[repr(i32)]
    #[derive(Copy, Clone, Eq, PartialEq, Debug)]
    pub enum VkBorderColor {
        VK_BORDER_COLOR_FLOAT_TRANSPARENT_BLACK = 0,
        VK_BORDER_COLOR_INT_TRANSPARENT_BLACK = 1,
        VK_BORDER_COLOR_FLOAT_OPAQUE_BLACK = 2,
        VK_BORDER_COLOR_INT_OPAQUE_BLACK = 3,
        VK_BORDER_COLOR_FLOAT_OPAQUE_WHITE = 4,
        VK_BORDER_COLOR_INT_OPAQUE_WHITE = 5,
    }

    #[repr(i32)]
    #[derive(Copy, Clone, Eq, PartialEq, Debug)]
    pub enum VkPipelineCacheHeaderVersion {
        VK_PIPELINE_CACHE_HEADER_VERSION_ONE = 1,
    }

    #[repr(i32)]
    #[derive(Copy, Clone, Eq, PartialEq, Debug)]
    pub enum VkBufferCreateFlagBits {
        VK_BUFFER_CREATE_SPARSE_BINDING_BIT = 1,
        VK_BUFFER_CREATE_SPARSE_RESIDENCY_BIT = 2,
        VK_BUFFER_CREATE_SPARSE_ALIASED_BIT = 4,
    }

    #[repr(i32)]
    #[derive(Copy, Clone, Eq, PartialEq, Debug)]
    pub enum VkBufferUsageFlagBits {
        VK_BUFFER_USAGE_TRANSFER_SRC_BIT = 1,
        VK_BUFFER_USAGE_TRANSFER_DST_BIT = 2,
        VK_BUFFER_USAGE_UNIFORM_TEXEL_BUFFER_BIT = 4,
        VK_BUFFER_USAGE_STORAGE_TEXEL_BUFFER_BIT = 8,
        VK_BUFFER_USAGE_UNIFORM_BUFFER_BIT = 16,
        VK_BUFFER_USAGE_STORAGE_BUFFER_BIT = 32,
        VK_BUFFER_USAGE_INDEX_BUFFER_BIT = 64,
        VK_BUFFER_USAGE_VERTEX_BUFFER_BIT = 128,
        VK_BUFFER_USAGE_INDIRECT_BUFFER_BIT = 256,
    }

    #[repr(i32)]
    #[derive(Copy, Clone, Eq, PartialEq, Debug)]
    pub enum VkColorComponentFlagBits {
        VK_COLOR_COMPONENT_R_BIT = 1,
        VK_COLOR_COMPONENT_G_BIT = 2,
        VK_COLOR_COMPONENT_B_BIT = 4,
        VK_COLOR_COMPONENT_A_BIT = 8,
    }

    #[repr(i32)]
    #[derive(Copy, Clone, Eq, PartialEq, Debug)]
    pub enum VkComponentSwizzle {
        VK_COMPONENT_SWIZZLE_IDENTITY = 0,
        VK_COMPONENT_SWIZZLE_ZERO = 1,
        VK_COMPONENT_SWIZZLE_ONE = 2,
        VK_COMPONENT_SWIZZLE_R = 3,
        VK_COMPONENT_SWIZZLE_G = 4,
        VK_COMPONENT_SWIZZLE_B = 5,
        VK_COMPONENT_SWIZZLE_A = 6,
    }

    #[repr(i32)]
    #[derive(Copy, Clone, Eq, PartialEq, Debug)]
    pub enum VkCommandPoolCreateFlagBits {
        VK_COMMAND_POOL_CREATE_TRANSIENT_BIT = 1,
        VK_COMMAND_POOL_CREATE_RESET_COMMAND_BUFFER_BIT = 2,
    }

    #[repr(i32)]
    #[derive(Copy, Clone, Eq, PartialEq, Debug)]
    pub enum VkCommandPoolResetFlagBits {
        VK_COMMAND_POOL_RESET_RELEASE_RESOURCES_BIT = 1,
    }

    #[repr(i32)]
    #[derive(Copy, Clone, Eq, PartialEq, Debug)]
    pub enum VkCommandBufferResetFlagBits {
        VK_COMMAND_BUFFER_RESET_RELEASE_RESOURCES_BIT = 1,
    }

    #[repr(i32)]
    #[derive(Copy, Clone, Eq, PartialEq, Debug)]
    pub enum VkCommandBufferLevel {
        VK_COMMAND_BUFFER_LEVEL_PRIMARY = 0,
        VK_COMMAND_BUFFER_LEVEL_SECONDARY = 1,
    }

    #[repr(i32)]
    #[derive(Copy, Clone, Eq, PartialEq, Debug)]
    pub enum VkCommandBufferUsageFlagBits {
        VK_COMMAND_BUFFER_USAGE_ONE_TIME_SUBMIT_BIT = 1,
        VK_COMMAND_BUFFER_USAGE_RENDER_PASS_CONTINUE_BIT = 2,
        VK_COMMAND_BUFFER_USAGE_SIMULTANEOUS_USE_BIT = 4,
    }

    #[repr(i32)]
    #[derive(Copy, Clone, Eq, PartialEq, Debug)]
    pub enum VkCompareOp {
        VK_COMPARE_OP_NEVER = 0,
        VK_COMPARE_OP_LESS = 1,
        VK_COMPARE_OP_EQUAL = 2,
        VK_COMPARE_OP_LESS_OR_EQUAL = 3,
        VK_COMPARE_OP_GREATER = 4,
        VK_COMPARE_OP_NOT_EQUAL = 5,
        VK_COMPARE_OP_GREATER_OR_EQUAL = 6,
        VK_COMPARE_OP_ALWAYS = 7,
    }

    #[repr(i32)]
    #[derive(Copy, Clone, Eq, PartialEq, Debug)]
    pub enum VkCullModeFlagBits {
        VK_CULL_MODE_NONE = 0,
        VK_CULL_MODE_FRONT_BIT = 1,
        VK_CULL_MODE_BACK_BIT = 2,
        VK_CULL_MODE_FRONT_AND_BACK = 0x00000003,
    }

    #[repr(i32)]
    #[derive(Copy, Clone, Eq, PartialEq, Debug)]
    pub enum VkDescriptorType {
        VK_DESCRIPTOR_TYPE_SAMPLER = 0,
        VK_DESCRIPTOR_TYPE_COMBINED_IMAGE_SAMPLER = 1,
        VK_DESCRIPTOR_TYPE_SAMPLED_IMAGE = 2,
        VK_DESCRIPTOR_TYPE_STORAGE_IMAGE = 3,
        VK_DESCRIPTOR_TYPE_UNIFORM_TEXEL_BUFFER = 4,
        VK_DESCRIPTOR_TYPE_STORAGE_TEXEL_BUFFER = 5,
        VK_DESCRIPTOR_TYPE_UNIFORM_BUFFER = 6,
        VK_DESCRIPTOR_TYPE_STORAGE_BUFFER = 7,
        VK_DESCRIPTOR_TYPE_UNIFORM_BUFFER_DYNAMIC = 8,
        VK_DESCRIPTOR_TYPE_STORAGE_BUFFER_DYNAMIC = 9,
        VK_DESCRIPTOR_TYPE_INPUT_ATTACHMENT = 10,
    }

    #[repr(i32)]
    #[derive(Copy, Clone, Eq, PartialEq, Debug)]
    pub enum VkDynamicState {
        VK_DYNAMIC_STATE_VIEWPORT = 0,
        VK_DYNAMIC_STATE_SCISSOR = 1,
        VK_DYNAMIC_STATE_LINE_WIDTH = 2,
        VK_DYNAMIC_STATE_DEPTH_BIAS = 3,
        VK_DYNAMIC_STATE_BLEND_CONSTANTS = 4,
        VK_DYNAMIC_STATE_DEPTH_BOUNDS = 5,
        VK_DYNAMIC_STATE_STENCIL_COMPARE_MASK = 6,
        VK_DYNAMIC_STATE_STENCIL_WRITE_MASK = 7,
        VK_DYNAMIC_STATE_STENCIL_REFERENCE = 8,
    }

    #[repr(i32)]
    #[derive(Copy, Clone, Eq, PartialEq, Debug)]
    pub enum VkFenceCreateFlagBits {
        VK_FENCE_CREATE_SIGNALED_BIT = 1,
    }

    #[repr(i32)]
    #[derive(Copy, Clone, Eq, PartialEq, Debug)]
    pub enum VkPolygonMode {
        VK_POLYGON_MODE_FILL = 0,
        VK_POLYGON_MODE_LINE = 1,
        VK_POLYGON_MODE_POINT = 2,
    }

    #[repr(i32)]
    #[derive(Copy, Clone, Eq, PartialEq, Debug)]
    pub enum VkFormat {
        VK_FORMAT_UNDEFINED = 0,
        VK_FORMAT_R4G4_UNORM_PACK8 = 1,
        VK_FORMAT_R4G4B4A4_UNORM_PACK16 = 2,
        VK_FORMAT_B4G4R4A4_UNORM_PACK16 = 3,
        VK_FORMAT_R5G6B5_UNORM_PACK16 = 4,
        VK_FORMAT_B5G6R5_UNORM_PACK16 = 5,
        VK_FORMAT_R5G5B5A1_UNORM_PACK16 = 6,
        VK_FORMAT_B5G5R5A1_UNORM_PACK16 = 7,
        VK_FORMAT_A1R5G5B5_UNORM_PACK16 = 8,
        VK_FORMAT_R8_UNORM = 9,
        VK_FORMAT_R8_SNORM = 10,
        VK_FORMAT_R8_USCALED = 11,
        VK_FORMAT_R8_SSCALED = 12,
        VK_FORMAT_R8_UINT = 13,
        VK_FORMAT_R8_SINT = 14,
        VK_FORMAT_R8_SRGB = 15,
        VK_FORMAT_R8G8_UNORM = 16,
        VK_FORMAT_R8G8_SNORM = 17,
        VK_FORMAT_R8G8_USCALED = 18,
        VK_FORMAT_R8G8_SSCALED = 19,
        VK_FORMAT_R8G8_UINT = 20,
        VK_FORMAT_R8G8_SINT = 21,
        VK_FORMAT_R8G8_SRGB = 22,
        VK_FORMAT_R8G8B8_UNORM = 23,
        VK_FORMAT_R8G8B8_SNORM = 24,
        VK_FORMAT_R8G8B8_USCALED = 25,
        VK_FORMAT_R8G8B8_SSCALED = 26,
        VK_FORMAT_R8G8B8_UINT = 27,
        VK_FORMAT_R8G8B8_SINT = 28,
        VK_FORMAT_R8G8B8_SRGB = 29,
        VK_FORMAT_B8G8R8_UNORM = 30,
        VK_FORMAT_B8G8R8_SNORM = 31,
        VK_FORMAT_B8G8R8_USCALED = 32,
        VK_FORMAT_B8G8R8_SSCALED = 33,
        VK_FORMAT_B8G8R8_UINT = 34,
        VK_FORMAT_B8G8R8_SINT = 35,
        VK_FORMAT_B8G8R8_SRGB = 36,
        VK_FORMAT_R8G8B8A8_UNORM = 37,
        VK_FORMAT_R8G8B8A8_SNORM = 38,
        VK_FORMAT_R8G8B8A8_USCALED = 39,
        VK_FORMAT_R8G8B8A8_SSCALED = 40,
        VK_FORMAT_R8G8B8A8_UINT = 41,
        VK_FORMAT_R8G8B8A8_SINT = 42,
        VK_FORMAT_R8G8B8A8_SRGB = 43,
        VK_FORMAT_B8G8R8A8_UNORM = 44,
        VK_FORMAT_B8G8R8A8_SNORM = 45,
        VK_FORMAT_B8G8R8A8_USCALED = 46,
        VK_FORMAT_B8G8R8A8_SSCALED = 47,
        VK_FORMAT_B8G8R8A8_UINT = 48,
        VK_FORMAT_B8G8R8A8_SINT = 49,
        VK_FORMAT_B8G8R8A8_SRGB = 50,
        VK_FORMAT_A8B8G8R8_UNORM_PACK32 = 51,
        VK_FORMAT_A8B8G8R8_SNORM_PACK32 = 52,
        VK_FORMAT_A8B8G8R8_USCALED_PACK32 = 53,
        VK_FORMAT_A8B8G8R8_SSCALED_PACK32 = 54,
        VK_FORMAT_A8B8G8R8_UINT_PACK32 = 55,
        VK_FORMAT_A8B8G8R8_SINT_PACK32 = 56,
        VK_FORMAT_A8B8G8R8_SRGB_PACK32 = 57,
        VK_FORMAT_A2R10G10B10_UNORM_PACK32 = 58,
        VK_FORMAT_A2R10G10B10_SNORM_PACK32 = 59,
        VK_FORMAT_A2R10G10B10_USCALED_PACK32 = 60,
        VK_FORMAT_A2R10G10B10_SSCALED_PACK32 = 61,
        VK_FORMAT_A2R10G10B10_UINT_PACK32 = 62,
        VK_FORMAT_A2R10G10B10_SINT_PACK32 = 63,
        VK_FORMAT_A2B10G10R10_UNORM_PACK32 = 64,
        VK_FORMAT_A2B10G10R10_SNORM_PACK32 = 65,
        VK_FORMAT_A2B10G10R10_USCALED_PACK32 = 66,
        VK_FORMAT_A2B10G10R10_SSCALED_PACK32 = 67,
        VK_FORMAT_A2B10G10R10_UINT_PACK32 = 68,
        VK_FORMAT_A2B10G10R10_SINT_PACK32 = 69,
        VK_FORMAT_R16_UNORM = 70,
        VK_FORMAT_R16_SNORM = 71,
        VK_FORMAT_R16_USCALED = 72,
        VK_FORMAT_R16_SSCALED = 73,
        VK_FORMAT_R16_UINT = 74,
        VK_FORMAT_R16_SINT = 75,
        VK_FORMAT_R16_SFLOAT = 76,
        VK_FORMAT_R16G16_UNORM = 77,
        VK_FORMAT_R16G16_SNORM = 78,
        VK_FORMAT_R16G16_USCALED = 79,
        VK_FORMAT_R16G16_SSCALED = 80,
        VK_FORMAT_R16G16_UINT = 81,
        VK_FORMAT_R16G16_SINT = 82,
        VK_FORMAT_R16G16_SFLOAT = 83,
        VK_FORMAT_R16G16B16_UNORM = 84,
        VK_FORMAT_R16G16B16_SNORM = 85,
        VK_FORMAT_R16G16B16_USCALED = 86,
        VK_FORMAT_R16G16B16_SSCALED = 87,
        VK_FORMAT_R16G16B16_UINT = 88,
        VK_FORMAT_R16G16B16_SINT = 89,
        VK_FORMAT_R16G16B16_SFLOAT = 90,
        VK_FORMAT_R16G16B16A16_UNORM = 91,
        VK_FORMAT_R16G16B16A16_SNORM = 92,
        VK_FORMAT_R16G16B16A16_USCALED = 93,
        VK_FORMAT_R16G16B16A16_SSCALED = 94,
        VK_FORMAT_R16G16B16A16_UINT = 95,
        VK_FORMAT_R16G16B16A16_SINT = 96,
        VK_FORMAT_R16G16B16A16_SFLOAT = 97,
        VK_FORMAT_R32_UINT = 98,
        VK_FORMAT_R32_SINT = 99,
        VK_FORMAT_R32_SFLOAT = 100,
        VK_FORMAT_R32G32_UINT = 101,
        VK_FORMAT_R32G32_SINT = 102,
        VK_FORMAT_R32G32_SFLOAT = 103,
        VK_FORMAT_R32G32B32_UINT = 104,
        VK_FORMAT_R32G32B32_SINT = 105,
        VK_FORMAT_R32G32B32_SFLOAT = 106,
        VK_FORMAT_R32G32B32A32_UINT = 107,
        VK_FORMAT_R32G32B32A32_SINT = 108,
        VK_FORMAT_R32G32B32A32_SFLOAT = 109,
        VK_FORMAT_R64_UINT = 110,
        VK_FORMAT_R64_SINT = 111,
        VK_FORMAT_R64_SFLOAT = 112,
        VK_FORMAT_R64G64_UINT = 113,
        VK_FORMAT_R64G64_SINT = 114,
        VK_FORMAT_R64G64_SFLOAT = 115,
        VK_FORMAT_R64G64B64_UINT = 116,
        VK_FORMAT_R64G64B64_SINT = 117,
        VK_FORMAT_R64G64B64_SFLOAT = 118,
        VK_FORMAT_R64G64B64A64_UINT = 119,
        VK_FORMAT_R64G64B64A64_SINT = 120,
        VK_FORMAT_R64G64B64A64_SFLOAT = 121,
        VK_FORMAT_B10G11R11_UFLOAT_PACK32 = 122,
        VK_FORMAT_E5B9G9R9_UFLOAT_PACK32 = 123,
        VK_FORMAT_D16_UNORM = 124,
        VK_FORMAT_X8_D24_UNORM_PACK32 = 125,
        VK_FORMAT_D32_SFLOAT = 126,
        VK_FORMAT_S8_UINT = 127,
        VK_FORMAT_D16_UNORM_S8_UINT = 128,
        VK_FORMAT_D24_UNORM_S8_UINT = 129,
        VK_FORMAT_D32_SFLOAT_S8_UINT = 130,
        VK_FORMAT_BC1_RGB_UNORM_BLOCK = 131,
        VK_FORMAT_BC1_RGB_SRGB_BLOCK = 132,
        VK_FORMAT_BC1_RGBA_UNORM_BLOCK = 133,
        VK_FORMAT_BC1_RGBA_SRGB_BLOCK = 134,
        VK_FORMAT_BC2_UNORM_BLOCK = 135,
        VK_FORMAT_BC2_SRGB_BLOCK = 136,
        VK_FORMAT_BC3_UNORM_BLOCK = 137,
        VK_FORMAT_BC3_SRGB_BLOCK = 138,
        VK_FORMAT_BC4_UNORM_BLOCK = 139,
        VK_FORMAT_BC4_SNORM_BLOCK = 140,
        VK_FORMAT_BC5_UNORM_BLOCK = 141,
        VK_FORMAT_BC5_SNORM_BLOCK = 142,
        VK_FORMAT_BC6H_UFLOAT_BLOCK = 143,
        VK_FORMAT_BC6H_SFLOAT_BLOCK = 144,
        VK_FORMAT_BC7_UNORM_BLOCK = 145,
        VK_FORMAT_BC7_SRGB_BLOCK = 146,
        VK_FORMAT_ETC2_R8G8B8_UNORM_BLOCK = 147,
        VK_FORMAT_ETC2_R8G8B8_SRGB_BLOCK = 148,
        VK_FORMAT_ETC2_R8G8B8A1_UNORM_BLOCK = 149,
        VK_FORMAT_ETC2_R8G8B8A1_SRGB_BLOCK = 150,
        VK_FORMAT_ETC2_R8G8B8A8_UNORM_BLOCK = 151,
        VK_FORMAT_ETC2_R8G8B8A8_SRGB_BLOCK = 152,
        VK_FORMAT_EAC_R11_UNORM_BLOCK = 153,
        VK_FORMAT_EAC_R11_SNORM_BLOCK = 154,
        VK_FORMAT_EAC_R11G11_UNORM_BLOCK = 155,
        VK_FORMAT_EAC_R11G11_SNORM_BLOCK = 156,
        VK_FORMAT_ASTC_4x4_UNORM_BLOCK = 157,
        VK_FORMAT_ASTC_4x4_SRGB_BLOCK = 158,
        VK_FORMAT_ASTC_5x4_UNORM_BLOCK = 159,
        VK_FORMAT_ASTC_5x4_SRGB_BLOCK = 160,
        VK_FORMAT_ASTC_5x5_UNORM_BLOCK = 161,
        VK_FORMAT_ASTC_5x5_SRGB_BLOCK = 162,
        VK_FORMAT_ASTC_6x5_UNORM_BLOCK = 163,
        VK_FORMAT_ASTC_6x5_SRGB_BLOCK = 164,
        VK_FORMAT_ASTC_6x6_UNORM_BLOCK = 165,
        VK_FORMAT_ASTC_6x6_SRGB_BLOCK = 166,
        VK_FORMAT_ASTC_8x5_UNORM_BLOCK = 167,
        VK_FORMAT_ASTC_8x5_SRGB_BLOCK = 168,
        VK_FORMAT_ASTC_8x6_UNORM_BLOCK = 169,
        VK_FORMAT_ASTC_8x6_SRGB_BLOCK = 170,
        VK_FORMAT_ASTC_8x8_UNORM_BLOCK = 171,
        VK_FORMAT_ASTC_8x8_SRGB_BLOCK = 172,
        VK_FORMAT_ASTC_10x5_UNORM_BLOCK = 173,
        VK_FORMAT_ASTC_10x5_SRGB_BLOCK = 174,
        VK_FORMAT_ASTC_10x6_UNORM_BLOCK = 175,
        VK_FORMAT_ASTC_10x6_SRGB_BLOCK = 176,
        VK_FORMAT_ASTC_10x8_UNORM_BLOCK = 177,
        VK_FORMAT_ASTC_10x8_SRGB_BLOCK = 178,
        VK_FORMAT_ASTC_10x10_UNORM_BLOCK = 179,
        VK_FORMAT_ASTC_10x10_SRGB_BLOCK = 180,
        VK_FORMAT_ASTC_12x10_UNORM_BLOCK = 181,
        VK_FORMAT_ASTC_12x10_SRGB_BLOCK = 182,
        VK_FORMAT_ASTC_12x12_UNORM_BLOCK = 183,
        VK_FORMAT_ASTC_12x12_SRGB_BLOCK = 184,
    }

    #[repr(i32)]
    #[derive(Copy, Clone, Eq, PartialEq, Debug)]
    pub enum VkFormatFeatureFlagBits {
        VK_FORMAT_FEATURE_SAMPLED_IMAGE_BIT = 1,
        VK_FORMAT_FEATURE_STORAGE_IMAGE_BIT = 2,
        VK_FORMAT_FEATURE_STORAGE_IMAGE_ATOMIC_BIT = 4,
        VK_FORMAT_FEATURE_UNIFORM_TEXEL_BUFFER_BIT = 8,
        VK_FORMAT_FEATURE_STORAGE_TEXEL_BUFFER_BIT = 16,
        VK_FORMAT_FEATURE_STORAGE_TEXEL_BUFFER_ATOMIC_BIT = 32,
        VK_FORMAT_FEATURE_VERTEX_BUFFER_BIT = 64,
        VK_FORMAT_FEATURE_COLOR_ATTACHMENT_BIT = 128,
        VK_FORMAT_FEATURE_COLOR_ATTACHMENT_BLEND_BIT = 256,
        VK_FORMAT_FEATURE_DEPTH_STENCIL_ATTACHMENT_BIT = 512,
        VK_FORMAT_FEATURE_BLIT_SRC_BIT = 1024,
        VK_FORMAT_FEATURE_BLIT_DST_BIT = 2048,
        VK_FORMAT_FEATURE_SAMPLED_IMAGE_FILTER_LINEAR_BIT = 4096,
    }

    #[repr(i32)]
    #[derive(Copy, Clone, Eq, PartialEq, Debug)]
    pub enum VkFrontFace {
        VK_FRONT_FACE_COUNTER_CLOCKWISE = 0,
        VK_FRONT_FACE_CLOCKWISE = 1,
    }

    #[repr(i32)]
    #[derive(Copy, Clone, Eq, PartialEq, Debug)]
    pub enum VkImageAspectFlagBits {
        VK_IMAGE_ASPECT_COLOR_BIT = 1,
        VK_IMAGE_ASPECT_DEPTH_BIT = 2,
        VK_IMAGE_ASPECT_STENCIL_BIT = 4,
        VK_IMAGE_ASPECT_METADATA_BIT = 8,
    }

    #[repr(i32)]
    #[derive(Copy, Clone, Eq, PartialEq, Debug)]
    pub enum VkImageCreateFlagBits {
        VK_IMAGE_CREATE_SPARSE_BINDING_BIT = 1,
        VK_IMAGE_CREATE_SPARSE_RESIDENCY_BIT = 2,
        VK_IMAGE_CREATE_SPARSE_ALIASED_BIT = 4,
        VK_IMAGE_CREATE_MUTABLE_FORMAT_BIT = 8,
        VK_IMAGE_CREATE_CUBE_COMPATIBLE_BIT = 16,
    }

    #[repr(i32)]
    #[derive(Copy, Clone, Eq, PartialEq, Debug)]
    pub enum VkImageLayout {
        VK_IMAGE_LAYOUT_UNDEFINED = 0,
        VK_IMAGE_LAYOUT_GENERAL = 1,
        VK_IMAGE_LAYOUT_COLOR_ATTACHMENT_OPTIMAL = 2,
        VK_IMAGE_LAYOUT_DEPTH_STENCIL_ATTACHMENT_OPTIMAL = 3,
        VK_IMAGE_LAYOUT_DEPTH_STENCIL_READ_ONLY_OPTIMAL = 4,
        VK_IMAGE_LAYOUT_SHADER_READ_ONLY_OPTIMAL = 5,
        VK_IMAGE_LAYOUT_TRANSFER_SRC_OPTIMAL = 6,
        VK_IMAGE_LAYOUT_TRANSFER_DST_OPTIMAL = 7,
        VK_IMAGE_LAYOUT_PREINITIALIZED = 8,
    }

    #[repr(i32)]
    #[derive(Copy, Clone, Eq, PartialEq, Debug)]
    pub enum VkImageTiling {
        VK_IMAGE_TILING_OPTIMAL = 0,
        VK_IMAGE_TILING_LINEAR = 1,
    }

    #[repr(i32)]
    #[derive(Copy, Clone, Eq, PartialEq, Debug)]
    pub enum VkImageType {
        VK_IMAGE_TYPE_1D = 0,
        VK_IMAGE_TYPE_2D = 1,
        VK_IMAGE_TYPE_3D = 2,
    }

    #[repr(i32)]
    #[derive(Copy, Clone, Eq, PartialEq, Debug)]
    pub enum VkImageUsageFlagBits {
        VK_IMAGE_USAGE_TRANSFER_SRC_BIT = 1,
        VK_IMAGE_USAGE_TRANSFER_DST_BIT = 2,
        VK_IMAGE_USAGE_SAMPLED_BIT = 4,
        VK_IMAGE_USAGE_STORAGE_BIT = 8,
        VK_IMAGE_USAGE_COLOR_ATTACHMENT_BIT = 16,
        VK_IMAGE_USAGE_DEPTH_STENCIL_ATTACHMENT_BIT = 32,
        VK_IMAGE_USAGE_TRANSIENT_ATTACHMENT_BIT = 64,
        VK_IMAGE_USAGE_INPUT_ATTACHMENT_BIT = 128,
    }

    #[repr(i32)]
    #[derive(Copy, Clone, Eq, PartialEq, Debug)]
    pub enum VkImageViewType {
        VK_IMAGE_VIEW_TYPE_1D = 0,
        VK_IMAGE_VIEW_TYPE_2D = 1,
        VK_IMAGE_VIEW_TYPE_3D = 2,
        VK_IMAGE_VIEW_TYPE_CUBE = 3,
        VK_IMAGE_VIEW_TYPE_1D_ARRAY = 4,
        VK_IMAGE_VIEW_TYPE_2D_ARRAY = 5,
        VK_IMAGE_VIEW_TYPE_CUBE_ARRAY = 6,
    }

    #[repr(i32)]
    #[derive(Copy, Clone, Eq, PartialEq, Debug)]
    pub enum VkSharingMode {
        VK_SHARING_MODE_EXCLUSIVE = 0,
        VK_SHARING_MODE_CONCURRENT = 1,
    }

    #[repr(i32)]
    #[derive(Copy, Clone, Eq, PartialEq, Debug)]
    pub enum VkIndexType {
        VK_INDEX_TYPE_UINT16 = 0,
        VK_INDEX_TYPE_UINT32 = 1,
    }

    #[repr(i32)]
    #[derive(Copy, Clone, Eq, PartialEq, Debug)]
    pub enum VkLogicOp {
        VK_LOGIC_OP_CLEAR = 0,
        VK_LOGIC_OP_AND = 1,
        VK_LOGIC_OP_AND_REVERSE = 2,
        VK_LOGIC_OP_COPY = 3,
        VK_LOGIC_OP_AND_INVERTED = 4,
        VK_LOGIC_OP_NO_OP = 5,
        VK_LOGIC_OP_XOR = 6,
        VK_LOGIC_OP_OR = 7,
        VK_LOGIC_OP_NOR = 8,
        VK_LOGIC_OP_EQUIVALENT = 9,
        VK_LOGIC_OP_INVERT = 10,
        VK_LOGIC_OP_OR_REVERSE = 11,
        VK_LOGIC_OP_COPY_INVERTED = 12,
        VK_LOGIC_OP_OR_INVERTED = 13,
        VK_LOGIC_OP_NAND = 14,
        VK_LOGIC_OP_SET = 15,
    }

    #[repr(i32)]
    #[derive(Copy, Clone, Eq, PartialEq, Debug)]
    pub enum VkMemoryHeapFlagBits {
        VK_MEMORY_HEAP_DEVICE_LOCAL_BIT = 1,
    }

    #[repr(i32)]
    #[derive(Copy, Clone, Eq, PartialEq, Debug)]
    pub enum VkAccessFlagBits {
        VK_ACCESS_INDIRECT_COMMAND_READ_BIT = 1,
        VK_ACCESS_INDEX_READ_BIT = 2,
        VK_ACCESS_VERTEX_ATTRIBUTE_READ_BIT = 4,
        VK_ACCESS_UNIFORM_READ_BIT = 8,
        VK_ACCESS_INPUT_ATTACHMENT_READ_BIT = 16,
        VK_ACCESS_SHADER_READ_BIT = 32,
        VK_ACCESS_SHADER_WRITE_BIT = 64,
        VK_ACCESS_COLOR_ATTACHMENT_READ_BIT = 128,
        VK_ACCESS_COLOR_ATTACHMENT_WRITE_BIT = 256,
        VK_ACCESS_DEPTH_STENCIL_ATTACHMENT_READ_BIT = 512,
        VK_ACCESS_DEPTH_STENCIL_ATTACHMENT_WRITE_BIT = 1024,
        VK_ACCESS_TRANSFER_READ_BIT = 2048,
        VK_ACCESS_TRANSFER_WRITE_BIT = 4096,
        VK_ACCESS_HOST_READ_BIT = 8192,
        VK_ACCESS_HOST_WRITE_BIT = 16384,
        VK_ACCESS_MEMORY_READ_BIT = 32768,
        VK_ACCESS_MEMORY_WRITE_BIT = 65536,
    }

    #[repr(i32)]
    #[derive(Copy, Clone, Eq, PartialEq, Debug)]
    pub enum VkMemoryPropertyFlagBits {
        VK_MEMORY_PROPERTY_DEVICE_LOCAL_BIT = 1,
        VK_MEMORY_PROPERTY_HOST_VISIBLE_BIT = 2,
        VK_MEMORY_PROPERTY_HOST_COHERENT_BIT = 4,
        VK_MEMORY_PROPERTY_HOST_CACHED_BIT = 8,
        VK_MEMORY_PROPERTY_LAZILY_ALLOCATED_BIT = 16,
    }

    #[repr(i32)]
    #[derive(Copy, Clone, Eq, PartialEq, Debug)]
    pub enum VkPhysicalDeviceType {
        VK_PHYSICAL_DEVICE_TYPE_OTHER = 0,
        VK_PHYSICAL_DEVICE_TYPE_INTEGRATED_GPU = 1,
        VK_PHYSICAL_DEVICE_TYPE_DISCRETE_GPU = 2,
        VK_PHYSICAL_DEVICE_TYPE_VIRTUAL_GPU = 3,
        VK_PHYSICAL_DEVICE_TYPE_CPU = 4,
    }

    #[repr(i32)]
    #[derive(Copy, Clone, Eq, PartialEq, Debug)]
    pub enum VkPipelineBindPoint {
        VK_PIPELINE_BIND_POINT_GRAPHICS = 0,
        VK_PIPELINE_BIND_POINT_COMPUTE = 1,
    }

    #[repr(i32)]
    #[derive(Copy, Clone, Eq, PartialEq, Debug)]
    pub enum VkPipelineCreateFlagBits {
        VK_PIPELINE_CREATE_DISABLE_OPTIMIZATION_BIT = 1,
        VK_PIPELINE_CREATE_ALLOW_DERIVATIVES_BIT = 2,
        VK_PIPELINE_CREATE_DERIVATIVE_BIT = 4,
    }

    #[repr(i32)]
    #[derive(Copy, Clone, Eq, PartialEq, Debug)]
    pub enum VkPrimitiveTopology {
        VK_PRIMITIVE_TOPOLOGY_POINT_LIST = 0,
        VK_PRIMITIVE_TOPOLOGY_LINE_LIST = 1,
        VK_PRIMITIVE_TOPOLOGY_LINE_STRIP = 2,
        VK_PRIMITIVE_TOPOLOGY_TRIANGLE_LIST = 3,
        VK_PRIMITIVE_TOPOLOGY_TRIANGLE_STRIP = 4,
        VK_PRIMITIVE_TOPOLOGY_TRIANGLE_FAN = 5,
        VK_PRIMITIVE_TOPOLOGY_LINE_LIST_WITH_ADJACENCY = 6,
        VK_PRIMITIVE_TOPOLOGY_LINE_STRIP_WITH_ADJACENCY = 7,
        VK_PRIMITIVE_TOPOLOGY_TRIANGLE_LIST_WITH_ADJACENCY = 8,
        VK_PRIMITIVE_TOPOLOGY_TRIANGLE_STRIP_WITH_ADJACENCY = 9,
        VK_PRIMITIVE_TOPOLOGY_PATCH_LIST = 10,
    }

    #[repr(i32)]
    #[derive(Copy, Clone, Eq, PartialEq, Debug)]
    pub enum VkQueryControlFlagBits {
        VK_QUERY_CONTROL_PRECISE_BIT = 1,
    }

    #[repr(i32)]
    #[derive(Copy, Clone, Eq, PartialEq, Debug)]
    pub enum VkQueryPipelineStatisticFlagBits {
        VK_QUERY_PIPELINE_STATISTIC_INPUT_ASSEMBLY_VERTICES_BIT = 1,
        VK_QUERY_PIPELINE_STATISTIC_INPUT_ASSEMBLY_PRIMITIVES_BIT = 2,
        VK_QUERY_PIPELINE_STATISTIC_VERTEX_SHADER_INVOCATIONS_BIT = 4,
        VK_QUERY_PIPELINE_STATISTIC_GEOMETRY_SHADER_INVOCATIONS_BIT = 8,
        VK_QUERY_PIPELINE_STATISTIC_GEOMETRY_SHADER_PRIMITIVES_BIT = 16,
        VK_QUERY_PIPELINE_STATISTIC_CLIPPING_INVOCATIONS_BIT = 32,
        VK_QUERY_PIPELINE_STATISTIC_CLIPPING_PRIMITIVES_BIT = 64,
        VK_QUERY_PIPELINE_STATISTIC_FRAGMENT_SHADER_INVOCATIONS_BIT = 128,
        VK_QUERY_PIPELINE_STATISTIC_TESSELLATION_CONTROL_SHADER_PATCHES_BIT = 256,
        VK_QUERY_PIPELINE_STATISTIC_TESSELLATION_EVALUATION_SHADER_INVOCATIONS_BIT = 512,
        VK_QUERY_PIPELINE_STATISTIC_COMPUTE_SHADER_INVOCATIONS_BIT = 1024,
    }

    #[repr(i32)]
    #[derive(Copy, Clone, Eq, PartialEq, Debug)]
    pub enum VkQueryResultFlagBits {
        VK_QUERY_RESULT_64_BIT = 1,
        VK_QUERY_RESULT_WAIT_BIT = 2,
        VK_QUERY_RESULT_WITH_AVAILABILITY_BIT = 4,
        VK_QUERY_RESULT_PARTIAL_BIT = 8,
    }

    #[repr(i32)]
    #[derive(Copy, Clone, Eq, PartialEq, Debug)]
    pub enum VkQueryType {
        VK_QUERY_TYPE_OCCLUSION = 0,
        VK_QUERY_TYPE_PIPELINE_STATISTICS = 1,
        VK_QUERY_TYPE_TIMESTAMP = 2,
    }

    #[repr(i32)]
    #[derive(Copy, Clone, Eq, PartialEq, Debug)]
    pub enum VkQueueFlagBits {
        VK_QUEUE_GRAPHICS_BIT = 1,
        VK_QUEUE_COMPUTE_BIT = 2,
        VK_QUEUE_TRANSFER_BIT = 4,
        VK_QUEUE_SPARSE_BINDING_BIT = 8,
    }

    #[repr(i32)]
    #[derive(Copy, Clone, Eq, PartialEq, Debug)]
    pub enum VkSubpassContents {
        VK_SUBPASS_CONTENTS_INLINE = 0,
        VK_SUBPASS_CONTENTS_SECONDARY_COMMAND_BUFFERS = 1,
    }

    #[repr(i32)]
    #[derive(Copy, Clone, Eq, PartialEq, Debug)]
    pub enum VkResult {
        VK_SUCCESS = 0,
        VK_NOT_READY = 1,
        VK_TIMEOUT = 2,
        VK_EVENT_SET = 3,
        VK_EVENT_RESET = 4,
        VK_INCOMPLETE = 5,
        VK_ERROR_OUT_OF_HOST_MEMORY = -1,
        VK_ERROR_OUT_OF_DEVICE_MEMORY = -2,
        VK_ERROR_INITIALIZATION_FAILED = -3,
        VK_ERROR_DEVICE_LOST = -4,
        VK_ERROR_MEMORY_MAP_FAILED = -5,
        VK_ERROR_LAYER_NOT_PRESENT = -6,
        VK_ERROR_EXTENSION_NOT_PRESENT = -7,
        VK_ERROR_FEATURE_NOT_PRESENT = -8,
        VK_ERROR_INCOMPATIBLE_DRIVER = -9,
        VK_ERROR_TOO_MANY_OBJECTS = -10,
        VK_ERROR_FORMAT_NOT_SUPPORTED = -11,
        VK_ERROR_FRAGMENTED_POOL = -12,
        VK_ERROR_UNKNOWN = -13,
        VK_ERROR_INVALID_EXTERNAL_HANDLE = -1000072003,
    }

    #[repr(i32)]
    #[derive(Copy, Clone, Eq, PartialEq, Debug)]
    pub enum VkShaderStageFlagBits {
        VK_SHADER_STAGE_VERTEX_BIT = 1,
        VK_SHADER_STAGE_TESSELLATION_CONTROL_BIT = 2,
        VK_SHADER_STAGE_TESSELLATION_EVALUATION_BIT = 4,
        VK_SHADER_STAGE_GEOMETRY_BIT = 8,
        VK_SHADER_STAGE_FRAGMENT_BIT = 16,
        VK_SHADER_STAGE_COMPUTE_BIT = 32,
        VK_SHADER_STAGE_ALL_GRAPHICS = 0x0000001F,
        VK_SHADER_STAGE_ALL = 0x7FFFFFFF,
    }

    #[repr(i32)]
    #[derive(Copy, Clone, Eq, PartialEq, Debug)]
    pub enum VkSparseMemoryBindFlagBits {
        VK_SPARSE_MEMORY_BIND_METADATA_BIT = 1,
    }

    #[repr(i32)]
    #[derive(Copy, Clone, Eq, PartialEq, Debug)]
    pub enum VkStencilFaceFlagBits {
        VK_STENCIL_FACE_FRONT_BIT = 1,
        VK_STENCIL_FACE_BACK_BIT = 2,
        VK_STENCIL_FACE_FRONT_AND_BACK = 0x00000003,
    }

    #[repr(i32)]
    #[derive(Copy, Clone, Eq, PartialEq, Debug)]
    pub enum VkStencilOp {
        VK_STENCIL_OP_KEEP = 0,
        VK_STENCIL_OP_ZERO = 1,
        VK_STENCIL_OP_REPLACE = 2,
        VK_STENCIL_OP_INCREMENT_AND_CLAMP = 3,
        VK_STENCIL_OP_DECREMENT_AND_CLAMP = 4,
        VK_STENCIL_OP_INVERT = 5,
        VK_STENCIL_OP_INCREMENT_AND_WRAP = 6,
        VK_STENCIL_OP_DECREMENT_AND_WRAP = 7,
    }

    #[repr(i32)]
    #[derive(Copy, Clone, Eq, PartialEq, Debug)]
    pub enum VkStructureType {
        VK_STRUCTURE_TYPE_APPLICATION_INFO = 0,
        VK_STRUCTURE_TYPE_INSTANCE_CREATE_INFO = 1,
        VK_STRUCTURE_TYPE_DEVICE_QUEUE_CREATE_INFO = 2,
        VK_STRUCTURE_TYPE_DEVICE_CREATE_INFO = 3,
        VK_STRUCTURE_TYPE_SUBMIT_INFO = 4,
        VK_STRUCTURE_TYPE_MEMORY_ALLOCATE_INFO = 5,
        VK_STRUCTURE_TYPE_MAPPED_MEMORY_RANGE = 6,
        VK_STRUCTURE_TYPE_BIND_SPARSE_INFO = 7,
        VK_STRUCTURE_TYPE_FENCE_CREATE_INFO = 8,
        VK_STRUCTURE_TYPE_SEMAPHORE_CREATE_INFO = 9,
        VK_STRUCTURE_TYPE_EVENT_CREATE_INFO = 10,
        VK_STRUCTURE_TYPE_QUERY_POOL_CREATE_INFO = 11,
        VK_STRUCTURE_TYPE_BUFFER_CREATE_INFO = 12,
        VK_STRUCTURE_TYPE_BUFFER_VIEW_CREATE_INFO = 13,
        VK_STRUCTURE_TYPE_IMAGE_CREATE_INFO = 14,
        VK_STRUCTURE_TYPE_IMAGE_VIEW_CREATE_INFO = 15,
        VK_STRUCTURE_TYPE_SHADER_MODULE_CREATE_INFO = 16,
        VK_STRUCTURE_TYPE_PIPELINE_CACHE_CREATE_INFO = 17,
        VK_STRUCTURE_TYPE_PIPELINE_SHADER_STAGE_CREATE_INFO = 18,
        VK_STRUCTURE_TYPE_PIPELINE_VERTEX_INPUT_STATE_CREATE_INFO = 19,
        VK_STRUCTURE_TYPE_PIPELINE_INPUT_ASSEMBLY_STATE_CREATE_INFO = 20,
        VK_STRUCTURE_TYPE_PIPELINE_TESSELLATION_STATE_CREATE_INFO = 21,
        VK_STRUCTURE_TYPE_PIPELINE_VIEWPORT_STATE_CREATE_INFO = 22,
        VK_STRUCTURE_TYPE_PIPELINE_RASTERIZATION_STATE_CREATE_INFO = 23,
        VK_STRUCTURE_TYPE_PIPELINE_MULTISAMPLE_STATE_CREATE_INFO = 24,
        VK_STRUCTURE_TYPE_PIPELINE_DEPTH_STENCIL_STATE_CREATE_INFO = 25,
        VK_STRUCTURE_TYPE_PIPELINE_COLOR_BLEND_STATE_CREATE_INFO = 26,
        VK_STRUCTURE_TYPE_PIPELINE_DYNAMIC_STATE_CREATE_INFO = 27,
        VK_STRUCTURE_TYPE_GRAPHICS_PIPELINE_CREATE_INFO = 28,
        VK_STRUCTURE_TYPE_COMPUTE_PIPELINE_CREATE_INFO = 29,
        VK_STRUCTURE_TYPE_PIPELINE_LAYOUT_CREATE_INFO = 30,
        VK_STRUCTURE_TYPE_SAMPLER_CREATE_INFO = 31,
        VK_STRUCTURE_TYPE_DESCRIPTOR_SET_LAYOUT_CREATE_INFO = 32,
        VK_STRUCTURE_TYPE_DESCRIPTOR_POOL_CREATE_INFO = 33,
        VK_STRUCTURE_TYPE_DESCRIPTOR_SET_ALLOCATE_INFO = 34,
        VK_STRUCTURE_TYPE_WRITE_DESCRIPTOR_SET = 35,
        VK_STRUCTURE_TYPE_COPY_DESCRIPTOR_SET = 36,
        VK_STRUCTURE_TYPE_FRAMEBUFFER_CREATE_INFO = 37,
        VK_STRUCTURE_TYPE_RENDER_PASS_CREATE_INFO = 38,
        VK_STRUCTURE_TYPE_COMMAND_POOL_CREATE_INFO = 39,
        VK_STRUCTURE_TYPE_COMMAND_BUFFER_ALLOCATE_INFO = 40,
        VK_STRUCTURE_TYPE_COMMAND_BUFFER_INHERITANCE_INFO = 41,
        VK_STRUCTURE_TYPE_COMMAND_BUFFER_BEGIN_INFO = 42,
        VK_STRUCTURE_TYPE_RENDER_PASS_BEGIN_INFO = 43,
        VK_STRUCTURE_TYPE_BUFFER_MEMORY_BARRIER = 44,
        VK_STRUCTURE_TYPE_IMAGE_MEMORY_BARRIER = 45,
        VK_STRUCTURE_TYPE_MEMORY_BARRIER = 46,
        VK_STRUCTURE_TYPE_LOADER_INSTANCE_CREATE_INFO = 47,
        VK_STRUCTURE_TYPE_LOADER_DEVICE_CREATE_INFO = 48,
        VK_STRUCTURE_TYPE_PHYSICAL_DEVICE_EXTERNAL_IMAGE_FORMAT_INFO = 1000071000,
        VK_STRUCTURE_TYPE_EXTERNAL_IMAGE_FORMAT_PROPERTIES = 1000071001,
        VK_STRUCTURE_TYPE_PHYSICAL_DEVICE_EXTERNAL_BUFFER_INFO = 1000071002,
        VK_STRUCTURE_TYPE_EXTERNAL_BUFFER_PROPERTIES = 1000071003,
        VK_STRUCTURE_TYPE_PHYSICAL_DEVICE_ID_PROPERTIES = 1000071004,
        VK_STRUCTURE_TYPE_EXTERNAL_MEMORY_BUFFER_CREATE_INFO = 1000072000,
        VK_STRUCTURE_TYPE_EXTERNAL_MEMORY_IMAGE_CREATE_INFO = 1000072001,
        VK_STRUCTURE_TYPE_EXPORT_MEMORY_ALLOCATE_INFO = 1000072002,
        VK_STRUCTURE_TYPE_IMPORT_MEMORY_WIN32_HANDLE_INFO_KHR = 1000073000,
        VK_STRUCTURE_TYPE_EXPORT_MEMORY_WIN32_HANDLE_INFO_KHR = 1000073001,
        VK_STRUCTURE_TYPE_MEMORY_WIN32_HANDLE_PROPERTIES_KHR = 1000073002,
        VK_STRUCTURE_TYPE_MEMORY_GET_WIN32_HANDLE_INFO_KHR = 1000073003,
    }

    #[repr(i32)]
    #[derive(Copy, Clone, Eq, PartialEq, Debug)]
    pub enum VkSystemAllocationScope {
        VK_SYSTEM_ALLOCATION_SCOPE_COMMAND = 0,
        VK_SYSTEM_ALLOCATION_SCOPE_OBJECT = 1,
        VK_SYSTEM_ALLOCATION_SCOPE_CACHE = 2,
        VK_SYSTEM_ALLOCATION_SCOPE_DEVICE = 3,
        VK_SYSTEM_ALLOCATION_SCOPE_INSTANCE = 4,
    }

    #[repr(i32)]
    #[derive(Copy, Clone, Eq, PartialEq, Debug)]
    pub enum VkInternalAllocationType {
        VK_INTERNAL_ALLOCATION_TYPE_EXECUTABLE = 0,
    }

    #[repr(i32)]
    #[derive(Copy, Clone, Eq, PartialEq, Debug)]
    pub enum VkSamplerAddressMode {
        VK_SAMPLER_ADDRESS_MODE_REPEAT = 0,
        VK_SAMPLER_ADDRESS_MODE_MIRRORED_REPEAT = 1,
        VK_SAMPLER_ADDRESS_MODE_CLAMP_TO_EDGE = 2,
        VK_SAMPLER_ADDRESS_MODE_CLAMP_TO_BORDER = 3,
    }

    #[repr(i32)]
    #[derive(Copy, Clone, Eq, PartialEq, Debug)]
    pub enum VkFilter {
        VK_FILTER_NEAREST = 0,
        VK_FILTER_LINEAR = 1,
    }

    #[repr(i32)]
    #[derive(Copy, Clone, Eq, PartialEq, Debug)]
    pub enum VkSamplerMipmapMode {
        VK_SAMPLER_MIPMAP_MODE_NEAREST = 0,
        VK_SAMPLER_MIPMAP_MODE_LINEAR = 1,
    }

    #[repr(i32)]
    #[derive(Copy, Clone, Eq, PartialEq, Debug)]
    pub enum VkVertexInputRate {
        VK_VERTEX_INPUT_RATE_VERTEX = 0,
        VK_VERTEX_INPUT_RATE_INSTANCE = 1,
    }

    #[repr(i32)]
    #[derive(Copy, Clone, Eq, PartialEq, Debug)]
    pub enum VkPipelineStageFlagBits {
        VK_PIPELINE_STAGE_TOP_OF_PIPE_BIT = 1,
        VK_PIPELINE_STAGE_DRAW_INDIRECT_BIT = 2,
        VK_PIPELINE_STAGE_VERTEX_INPUT_BIT = 4,
        VK_PIPELINE_STAGE_VERTEX_SHADER_BIT = 8,
        VK_PIPELINE_STAGE_TESSELLATION_CONTROL_SHADER_BIT = 16,
        VK_PIPELINE_STAGE_TESSELLATION_EVALUATION_SHADER_BIT = 32,
        VK_PIPELINE_STAGE_GEOMETRY_SHADER_BIT = 64,
        VK_PIPELINE_STAGE_FRAGMENT_SHADER_BIT = 128,
        VK_PIPELINE_STAGE_EARLY_FRAGMENT_TESTS_BIT = 256,
        VK_PIPELINE_STAGE_LATE_FRAGMENT_TESTS_BIT = 512,
        VK_PIPELINE_STAGE_COLOR_ATTACHMENT_OUTPUT_BIT = 1024,
        VK_PIPELINE_STAGE_COMPUTE_SHADER_BIT = 2048,
        VK_PIPELINE_STAGE_TRANSFER_BIT = 4096,
        VK_PIPELINE_STAGE_BOTTOM_OF_PIPE_BIT = 8192,
        VK_PIPELINE_STAGE_HOST_BIT = 16384,
        VK_PIPELINE_STAGE_ALL_GRAPHICS_BIT = 32768,
        VK_PIPELINE_STAGE_ALL_COMMANDS_BIT = 65536,
    }

    #[repr(i32)]
    #[derive(Copy, Clone, Eq, PartialEq, Debug)]
    pub enum VkSparseImageFormatFlagBits {
        VK_SPARSE_IMAGE_FORMAT_SINGLE_MIPTAIL_BIT = 1,
        VK_SPARSE_IMAGE_FORMAT_ALIGNED_MIP_SIZE_BIT = 2,
        VK_SPARSE_IMAGE_FORMAT_NONSTANDARD_BLOCK_SIZE_BIT = 4,
    }

    #[repr(i32)]
    #[derive(Copy, Clone, Eq, PartialEq, Debug)]
    pub enum VkSampleCountFlagBits {
        VK_SAMPLE_COUNT_1_BIT = 1,
        VK_SAMPLE_COUNT_2_BIT = 2,
        VK_SAMPLE_COUNT_4_BIT = 4,
        VK_SAMPLE_COUNT_8_BIT = 8,
        VK_SAMPLE_COUNT_16_BIT = 16,
        VK_SAMPLE_COUNT_32_BIT = 32,
        VK_SAMPLE_COUNT_64_BIT = 64,
    }

    #[repr(i32)]
    #[derive(Copy, Clone, Eq, PartialEq, Debug)]
    pub enum VkAttachmentDescriptionFlagBits {
        VK_ATTACHMENT_DESCRIPTION_MAY_ALIAS_BIT = 1,
    }

    #[repr(i32)]
    #[derive(Copy, Clone, Eq, PartialEq, Debug)]
    pub enum VkDescriptorPoolCreateFlagBits {
        VK_DESCRIPTOR_POOL_CREATE_FREE_DESCRIPTOR_SET_BIT = 1,
    }

    #[repr(i32)]
    #[derive(Copy, Clone, Eq, PartialEq, Debug)]
    pub enum VkDependencyFlagBits {
        VK_DEPENDENCY_BY_REGION_BIT = 1,
    }

    #[repr(i32)]
    #[derive(Copy, Clone, Eq, PartialEq, Debug)]
    pub enum VkObjectType {
        VK_OBJECT_TYPE_UNKNOWN = 0,
        VK_OBJECT_TYPE_INSTANCE = 1,
        VK_OBJECT_TYPE_PHYSICAL_DEVICE = 2,
        VK_OBJECT_TYPE_DEVICE = 3,
        VK_OBJECT_TYPE_QUEUE = 4,
        VK_OBJECT_TYPE_SEMAPHORE = 5,
        VK_OBJECT_TYPE_COMMAND_BUFFER = 6,
        VK_OBJECT_TYPE_FENCE = 7,
        VK_OBJECT_TYPE_DEVICE_MEMORY = 8,
        VK_OBJECT_TYPE_BUFFER = 9,
        VK_OBJECT_TYPE_IMAGE = 10,
        VK_OBJECT_TYPE_EVENT = 11,
        VK_OBJECT_TYPE_QUERY_POOL = 12,
        VK_OBJECT_TYPE_BUFFER_VIEW = 13,
        VK_OBJECT_TYPE_IMAGE_VIEW = 14,
        VK_OBJECT_TYPE_SHADER_MODULE = 15,
        VK_OBJECT_TYPE_PIPELINE_CACHE = 16,
        VK_OBJECT_TYPE_PIPELINE_LAYOUT = 17,
        VK_OBJECT_TYPE_RENDER_PASS = 18,
        VK_OBJECT_TYPE_PIPELINE = 19,
        VK_OBJECT_TYPE_DESCRIPTOR_SET_LAYOUT = 20,
        VK_OBJECT_TYPE_SAMPLER = 21,
        VK_OBJECT_TYPE_DESCRIPTOR_POOL = 22,
        VK_OBJECT_TYPE_DESCRIPTOR_SET = 23,
        VK_OBJECT_TYPE_FRAMEBUFFER = 24,
        VK_OBJECT_TYPE_COMMAND_POOL = 25,
    }

    #[repr(i32)]
    #[derive(Copy, Clone, Eq, PartialEq, Debug)]
    pub enum VkExternalMemoryHandleTypeFlagBits {
        VK_EXTERNAL_MEMORY_HANDLE_TYPE_OPAQUE_FD_BIT = 1,
        VK_EXTERNAL_MEMORY_HANDLE_TYPE_OPAQUE_WIN32_BIT = 2,
        VK_EXTERNAL_MEMORY_HANDLE_TYPE_OPAQUE_WIN32_KMT_BIT = 4,
        VK_EXTERNAL_MEMORY_HANDLE_TYPE_D3D11_TEXTURE_BIT = 8,
        VK_EXTERNAL_MEMORY_HANDLE_TYPE_D3D11_TEXTURE_KMT_BIT = 16,
        VK_EXTERNAL_MEMORY_HANDLE_TYPE_D3D12_HEAP_BIT = 32,
        VK_EXTERNAL_MEMORY_HANDLE_TYPE_D3D12_RESOURCE_BIT = 64,
    }
    pub type VkExternalMemoryHandleTypeFlagBitsKHR = VkExternalMemoryHandleTypeFlagBits;

    #[repr(i32)]
    #[derive(Copy, Clone, Eq, PartialEq, Debug)]
    pub enum VkExternalMemoryFeatureFlagBits {
        VK_EXTERNAL_MEMORY_FEATURE_DEDICATED_ONLY_BIT = 1,
        VK_EXTERNAL_MEMORY_FEATURE_EXPORTABLE_BIT = 2,
        VK_EXTERNAL_MEMORY_FEATURE_IMPORTABLE_BIT = 4,
    }
    pub type VkExternalMemoryFeatureFlagBitsKHR = VkExternalMemoryFeatureFlagBits;

    #[repr(i32)]
    #[derive(Copy, Clone, Eq, PartialEq, Debug)]
    pub enum VkVendorId {
        VK_VENDOR_ID_KHRONOS = 0x10000,
        VK_VENDOR_ID_VIV = 0x10001,
        VK_VENDOR_ID_VSI = 0x10002,
        VK_VENDOR_ID_KAZAN = 0x10003,
        VK_VENDOR_ID_CODEPLAY = 0x10004,
        VK_VENDOR_ID_MESA = 0x10005,
        VK_VENDOR_ID_POCL = 0x10006,
        VK_VENDOR_ID_MOBILEYE = 0x10007,
    }
    pub type PFN_vkInternalAllocationNotification = extern "system" fn(
        pUserData: *mut c_void,
        size: usize,
        allocationType: VkInternalAllocationType,
        allocationScope: VkSystemAllocationScope,
    ) -> ();
    pub type PFN_vkInternalFreeNotification = extern "system" fn(
        pUserData: *mut c_void,
        size: usize,
        allocationType: VkInternalAllocationType,
        allocationScope: VkSystemAllocationScope,
    ) -> ();
    pub type PFN_vkReallocationFunction = extern "system" fn(
        pUserData: *mut c_void,
        pOriginal: *mut c_void,
        size: usize,
        alignment: usize,
        allocationScope: VkSystemAllocationScope,
    ) -> *mut c_void;
    pub type PFN_vkAllocationFunction = extern "system" fn(
        pUserData: *mut c_void,
        size: usize,
        alignment: usize,
        allocationScope: VkSystemAllocationScope,
    ) -> *mut c_void;
    pub type PFN_vkFreeFunction =
        extern "system" fn(pUserData: *mut c_void, pMemory: *mut c_void) -> ();
    pub type PFN_vkVoidFunction = extern "system" fn() -> ();

    #[allow(non_snake_case)]
    #[repr(C)]
    #[derive(Copy, Clone)]
    pub struct VkBaseOutStructure {
        sType: VkStructureType,
        pNext: *mut VkBaseOutStructure,
    }

    #[allow(non_snake_case)]
    #[repr(C)]
    #[derive(Copy, Clone)]
    pub struct VkBaseInStructure {
        sType: VkStructureType,
        pNext: *const VkBaseInStructure,
    }

    #[allow(non_snake_case)]
    #[repr(C)]
    #[derive(Copy, Clone)]
    pub struct VkOffset2D {
        x: i32,
        y: i32,
    }

    #[allow(non_snake_case)]
    #[repr(C)]
    #[derive(Copy, Clone)]
    pub struct VkOffset3D {
        x: i32,
        y: i32,
        z: i32,
    }

    #[allow(non_snake_case)]
    #[repr(C)]
    #[derive(Copy, Clone)]
    pub struct VkExtent2D {
        width: u32,
        height: u32,
    }

    #[allow(non_snake_case)]
    #[repr(C)]
    #[derive(Copy, Clone)]
    pub struct VkExtent3D {
        width: u32,
        height: u32,
        depth: u32,
    }

    #[allow(non_snake_case)]
    #[repr(C)]
    #[derive(Copy, Clone)]
    pub struct VkViewport {
        x: c_float,
        y: c_float,
        width: c_float,
        height: c_float,
        minDepth: c_float,
        maxDepth: c_float,
    }

    #[allow(non_snake_case)]
    #[repr(C)]
    #[derive(Copy, Clone)]
    pub struct VkRect2D {
        offset: VkOffset2D,
        extent: VkExtent2D,
    }

    #[allow(non_snake_case)]
    #[repr(C)]
    #[derive(Copy, Clone)]
    pub struct VkClearRect {
        rect: VkRect2D,
        baseArrayLayer: u32,
        layerCount: u32,
    }

    #[allow(non_snake_case)]
    #[repr(C)]
    #[derive(Copy, Clone)]
    pub struct VkComponentMapping {
        r: VkComponentSwizzle,
        g: VkComponentSwizzle,
        b: VkComponentSwizzle,
        a: VkComponentSwizzle,
    }

    #[allow(non_snake_case)]
    #[repr(C)]
    #[derive(Copy, Clone)]
    pub struct VkExtensionProperties {
        extensionName: c_char,
        specVersion: u32,
    }

    #[allow(non_snake_case)]
    #[repr(C)]
    #[derive(Copy, Clone)]
    pub struct VkLayerProperties {
        layerName: c_char,
        specVersion: u32,
        implementationVersion: u32,
        description: c_char,
    }

    #[allow(non_snake_case)]
    #[repr(C)]
    #[derive(Copy, Clone)]
    pub struct VkApplicationInfo {
        sType: VkStructureType,
        pNext: *const c_void,
        pApplicationName: *const c_char,
        applicationVersion: u32,
        pEngineName: *const c_char,
        engineVersion: u32,
        apiVersion: u32,
    }

    #[allow(non_snake_case)]
    #[repr(C)]
    #[derive(Copy, Clone)]
    pub struct VkAllocationCallbacks {
        pUserData: *mut c_void,
        pfnAllocation: PFN_vkAllocationFunction,
        pfnReallocation: PFN_vkReallocationFunction,
        pfnFree: PFN_vkFreeFunction,
        pfnInternalAllocation: PFN_vkInternalAllocationNotification,
        pfnInternalFree: PFN_vkInternalFreeNotification,
    }

    #[allow(non_snake_case)]
    #[repr(C)]
    #[derive(Copy, Clone)]
    pub struct VkDescriptorImageInfo {
        sampler: VkSampler,
        imageView: VkImageView,
        imageLayout: VkImageLayout,
    }

    #[allow(non_snake_case)]
    #[repr(C)]
    #[derive(Copy, Clone)]
    pub struct VkCopyDescriptorSet {
        sType: VkStructureType,
        pNext: *const c_void,
        srcSet: VkDescriptorSet,
        srcBinding: u32,
        srcArrayElement: u32,
        dstSet: VkDescriptorSet,
        dstBinding: u32,
        dstArrayElement: u32,
        descriptorCount: u32,
    }

    #[allow(non_snake_case)]
    #[repr(C)]
    #[derive(Copy, Clone)]
    pub struct VkDescriptorPoolSize {
        type_: VkDescriptorType,
        descriptorCount: u32,
    }

    #[allow(non_snake_case)]
    #[repr(C)]
    #[derive(Copy, Clone)]
    pub struct VkDescriptorSetAllocateInfo {
        sType: VkStructureType,
        pNext: *const c_void,
        descriptorPool: VkDescriptorPool,
        descriptorSetCount: u32,
        pSetLayouts: *const VkDescriptorSetLayout,
    }

    #[allow(non_snake_case)]
    #[repr(C)]
    #[derive(Copy, Clone)]
    pub struct VkSpecializationMapEntry {
        constantID: u32,
        offset: u32,
        size: usize,
    }

    #[allow(non_snake_case)]
    #[repr(C)]
    #[derive(Copy, Clone)]
    pub struct VkSpecializationInfo {
        mapEntryCount: u32,
        pMapEntries: *const VkSpecializationMapEntry,
        dataSize: usize,
        pData: *const c_void,
    }

    #[allow(non_snake_case)]
    #[repr(C)]
    #[derive(Copy, Clone)]
    pub struct VkVertexInputBindingDescription {
        binding: u32,
        stride: u32,
        inputRate: VkVertexInputRate,
    }

    #[allow(non_snake_case)]
    #[repr(C)]
    #[derive(Copy, Clone)]
    pub struct VkVertexInputAttributeDescription {
        location: u32,
        binding: u32,
        format: VkFormat,
        offset: u32,
    }

    #[allow(non_snake_case)]
    #[repr(C)]
    #[derive(Copy, Clone)]
    pub struct VkStencilOpState {
        failOp: VkStencilOp,
        passOp: VkStencilOp,
        depthFailOp: VkStencilOp,
        compareOp: VkCompareOp,
        compareMask: u32,
        writeMask: u32,
        reference: u32,
    }

    #[allow(non_snake_case)]
    #[repr(C)]
    #[derive(Copy, Clone)]
    pub struct VkPipelineCacheHeaderVersionOne {
        headerSize: u32,
        headerVersion: VkPipelineCacheHeaderVersion,
        vendorID: u32,
        deviceID: u32,
        pipelineCacheUUID: u8,
    }

    #[allow(non_snake_case)]
    #[repr(C)]
    #[derive(Copy, Clone)]
    pub struct VkCommandBufferAllocateInfo {
        sType: VkStructureType,
        pNext: *const c_void,
        commandPool: VkCommandPool,
        level: VkCommandBufferLevel,
        commandBufferCount: u32,
    }

    #[allow(non_snake_case)]
    #[repr(C)]
    #[derive(Copy, Clone)]
    pub union VkClearColorValue {
        float32: [c_float; 4],
        int32: [i32; 4],
        uint32: [u32; 4],
    }

    #[allow(non_snake_case)]
    #[repr(C)]
    #[derive(Copy, Clone)]
    pub struct VkClearDepthStencilValue {
        depth: c_float,
        stencil: u32,
    }

    #[allow(non_snake_case)]
    #[repr(C)]
    #[derive(Copy, Clone)]
    pub union VkClearValue {
        color: VkClearColorValue,
        depthStencil: VkClearDepthStencilValue,
    }

    #[allow(non_snake_case)]
    #[repr(C)]
    #[derive(Copy, Clone)]
    pub struct VkAttachmentReference {
        attachment: u32,
        layout: VkImageLayout,
    }

    #[allow(non_snake_case)]
    #[repr(C)]
    #[derive(Copy, Clone)]
    pub struct VkDrawIndirectCommand {
        vertexCount: u32,
        instanceCount: u32,
        firstVertex: u32,
        firstInstance: u32,
    }

    #[allow(non_snake_case)]
    #[repr(C)]
    #[derive(Copy, Clone)]
    pub struct VkDrawIndexedIndirectCommand {
        indexCount: u32,
        instanceCount: u32,
        firstIndex: u32,
        vertexOffset: i32,
        firstInstance: u32,
    }

    #[allow(non_snake_case)]
    #[repr(C)]
    #[derive(Copy, Clone)]
    pub struct VkDispatchIndirectCommand {
        x: u32,
        y: u32,
        z: u32,
    }

    #[allow(non_snake_case)]
    #[repr(C)]
    #[derive(Copy, Clone)]
    pub struct VkPhysicalDeviceExternalImageFormatInfo {
        sType: VkStructureType,
        pNext: *const c_void,
        handleType: VkExternalMemoryHandleTypeFlagBits,
    }
    pub type VkPhysicalDeviceExternalImageFormatInfoKHR = VkPhysicalDeviceExternalImageFormatInfo;
    #[cfg(any(feature = "VK_USE_PLATFORM_WIN32_KHR"))]
    #[allow(non_snake_case)]
    #[repr(C)]
    #[derive(Copy, Clone)]
    pub struct VkImportMemoryWin32HandleInfoKHR {
        sType: VkStructureType,
        pNext: *const c_void,
        handleType: VkExternalMemoryHandleTypeFlagBits,
        handle: HANDLE,
        name: LPCWSTR,
    }
    #[cfg(any(feature = "VK_USE_PLATFORM_WIN32_KHR"))]
    #[allow(non_snake_case)]
    #[repr(C)]
    #[derive(Copy, Clone)]
    pub struct VkExportMemoryWin32HandleInfoKHR {
        sType: VkStructureType,
        pNext: *const c_void,
        pAttributes: *const SECURITY_ATTRIBUTES,
        dwAccess: DWORD,
        name: LPCWSTR,
    }
    #[cfg(any(feature = "VK_USE_PLATFORM_WIN32_KHR"))]
    #[allow(non_snake_case)]
    #[repr(C)]
    #[derive(Copy, Clone)]
    pub struct VkMemoryWin32HandlePropertiesKHR {
        sType: VkStructureType,
        pNext: *mut c_void,
        memoryTypeBits: u32,
    }
    #[cfg(any(feature = "VK_USE_PLATFORM_WIN32_KHR"))]
    #[allow(non_snake_case)]
    #[repr(C)]
    #[derive(Copy, Clone)]
    pub struct VkMemoryGetWin32HandleInfoKHR {
        sType: VkStructureType,
        pNext: *const c_void,
        memory: VkDeviceMemory,
        handleType: VkExternalMemoryHandleTypeFlagBits,
    }
    pub type VkSampleMask = u32;
    pub type VkBool32 = u32;
    pub type VkFlags = u32;
    pub type VkDeviceSize = u64;
    pub type VkDeviceAddress = u64;
    pub type VkFramebufferCreateFlags = VkFlags;
    pub type VkQueryPoolCreateFlags = VkFlags;
    pub type VkRenderPassCreateFlags = VkFlags;
    pub type VkSamplerCreateFlags = VkFlags;
    pub type VkPipelineLayoutCreateFlags = VkFlags;
    pub type VkPipelineCacheCreateFlags = VkFlags;
    pub type VkPipelineDepthStencilStateCreateFlags = VkFlags;
    pub type VkPipelineDynamicStateCreateFlags = VkFlags;
    pub type VkPipelineColorBlendStateCreateFlags = VkFlags;
    pub type VkPipelineMultisampleStateCreateFlags = VkFlags;
    pub type VkPipelineRasterizationStateCreateFlags = VkFlags;
    pub type VkPipelineViewportStateCreateFlags = VkFlags;
    pub type VkPipelineTessellationStateCreateFlags = VkFlags;
    pub type VkPipelineInputAssemblyStateCreateFlags = VkFlags;
    pub type VkPipelineVertexInputStateCreateFlags = VkFlags;
    pub type VkPipelineShaderStageCreateFlags = VkFlags;
    pub type VkDescriptorSetLayoutCreateFlags = VkFlags;
    pub type VkBufferViewCreateFlags = VkFlags;
    pub type VkInstanceCreateFlags = VkFlags;
    pub type VkDeviceCreateFlags = VkFlags;
    pub type VkDeviceQueueCreateFlags = VkFlags;
    pub type VkQueueFlags = VkFlags;
    pub type VkMemoryPropertyFlags = VkFlags;
    pub type VkMemoryHeapFlags = VkFlags;
    pub type VkAccessFlags = VkFlags;
    pub type VkBufferUsageFlags = VkFlags;
    pub type VkBufferCreateFlags = VkFlags;
    pub type VkShaderStageFlags = VkFlags;
    pub type VkImageUsageFlags = VkFlags;
    pub type VkImageCreateFlags = VkFlags;
    pub type VkImageViewCreateFlags = VkFlags;
    pub type VkPipelineCreateFlags = VkFlags;
    pub type VkColorComponentFlags = VkFlags;
    pub type VkFenceCreateFlags = VkFlags;
    pub type VkSemaphoreCreateFlags = VkFlags;
    pub type VkFormatFeatureFlags = VkFlags;
    pub type VkQueryControlFlags = VkFlags;
    pub type VkQueryResultFlags = VkFlags;
    pub type VkShaderModuleCreateFlags = VkFlags;
    pub type VkEventCreateFlags = VkFlags;
    pub type VkCommandPoolCreateFlags = VkFlags;
    pub type VkCommandPoolResetFlags = VkFlags;
    pub type VkCommandBufferResetFlags = VkFlags;
    pub type VkCommandBufferUsageFlags = VkFlags;
    pub type VkQueryPipelineStatisticFlags = VkFlags;
    pub type VkMemoryMapFlags = VkFlags;
    pub type VkImageAspectFlags = VkFlags;
    pub type VkSparseMemoryBindFlags = VkFlags;
    pub type VkSparseImageFormatFlags = VkFlags;
    pub type VkSubpassDescriptionFlags = VkFlags;
    pub type VkPipelineStageFlags = VkFlags;
    pub type VkSampleCountFlags = VkFlags;
    pub type VkAttachmentDescriptionFlags = VkFlags;
    pub type VkStencilFaceFlags = VkFlags;
    pub type VkCullModeFlags = VkFlags;
    pub type VkDescriptorPoolCreateFlags = VkFlags;
    pub type VkDescriptorPoolResetFlags = VkFlags;
    pub type VkDependencyFlags = VkFlags;
    pub type VkExternalMemoryHandleTypeFlags = VkFlags;
    pub type VkExternalMemoryHandleTypeFlagsKHR = VkExternalMemoryHandleTypeFlags;
    pub type VkExternalMemoryFeatureFlags = VkFlags;
    pub type VkExternalMemoryFeatureFlagsKHR = VkExternalMemoryFeatureFlags;

    #[allow(non_snake_case)]
    #[repr(C)]
    #[derive(Copy, Clone)]
    pub struct VkDeviceQueueCreateInfo {
        sType: VkStructureType,
        pNext: *const c_void,
        flags: VkDeviceQueueCreateFlags,
        queueFamilyIndex: u32,
        queueCount: u32,
        pQueuePriorities: *const c_float,
    }

    #[allow(non_snake_case)]
    #[repr(C)]
    #[derive(Copy, Clone)]
    pub struct VkInstanceCreateInfo {
        sType: VkStructureType,
        pNext: *const c_void,
        flags: VkInstanceCreateFlags,
        pApplicationInfo: *const VkApplicationInfo,
        enabledLayerCount: u32,
        ppEnabledLayerNames: *const *const c_char,
        enabledExtensionCount: u32,
        ppEnabledExtensionNames: *const *const c_char,
    }

    #[allow(non_snake_case)]
    #[repr(C)]
    #[derive(Copy, Clone)]
    pub struct VkQueueFamilyProperties {
        queueFlags: VkQueueFlags,
        queueCount: u32,
        timestampValidBits: u32,
        minImageTransferGranularity: VkExtent3D,
    }

    #[allow(non_snake_case)]
    #[repr(C)]
    #[derive(Copy, Clone)]
    pub struct VkMemoryAllocateInfo {
        sType: VkStructureType,
        pNext: *const c_void,
        allocationSize: VkDeviceSize,
        memoryTypeIndex: u32,
    }

    #[allow(non_snake_case)]
    #[repr(C)]
    #[derive(Copy, Clone)]
    pub struct VkMemoryRequirements {
        size: VkDeviceSize,
        alignment: VkDeviceSize,
        memoryTypeBits: u32,
    }

    #[allow(non_snake_case)]
    #[repr(C)]
    #[derive(Copy, Clone)]
    pub struct VkSparseImageFormatProperties {
        aspectMask: VkImageAspectFlags,
        imageGranularity: VkExtent3D,
        flags: VkSparseImageFormatFlags,
    }

    #[allow(non_snake_case)]
    #[repr(C)]
    #[derive(Copy, Clone)]
    pub struct VkSparseImageMemoryRequirements {
        formatProperties: VkSparseImageFormatProperties,
        imageMipTailFirstLod: u32,
        imageMipTailSize: VkDeviceSize,
        imageMipTailOffset: VkDeviceSize,
        imageMipTailStride: VkDeviceSize,
    }

    #[allow(non_snake_case)]
    #[repr(C)]
    #[derive(Copy, Clone)]
    pub struct VkMemoryType {
        propertyFlags: VkMemoryPropertyFlags,
        heapIndex: u32,
    }

    #[allow(non_snake_case)]
    #[repr(C)]
    #[derive(Copy, Clone)]
    pub struct VkMemoryHeap {
        size: VkDeviceSize,
        flags: VkMemoryHeapFlags,
    }

    #[allow(non_snake_case)]
    #[repr(C)]
    #[derive(Copy, Clone)]
    pub struct VkMappedMemoryRange {
        sType: VkStructureType,
        pNext: *const c_void,
        memory: VkDeviceMemory,
        offset: VkDeviceSize,
        size: VkDeviceSize,
    }

    #[allow(non_snake_case)]
    #[repr(C)]
    #[derive(Copy, Clone)]
    pub struct VkFormatProperties {
        linearTilingFeatures: VkFormatFeatureFlags,
        optimalTilingFeatures: VkFormatFeatureFlags,
        bufferFeatures: VkFormatFeatureFlags,
    }

    #[allow(non_snake_case)]
    #[repr(C)]
    #[derive(Copy, Clone)]
    pub struct VkImageFormatProperties {
        maxExtent: VkExtent3D,
        maxMipLevels: u32,
        maxArrayLayers: u32,
        sampleCounts: VkSampleCountFlags,
        maxResourceSize: VkDeviceSize,
    }

    #[allow(non_snake_case)]
    #[repr(C)]
    #[derive(Copy, Clone)]
    pub struct VkDescriptorBufferInfo {
        buffer: VkBuffer,
        offset: VkDeviceSize,
        range: VkDeviceSize,
    }

    #[allow(non_snake_case)]
    #[repr(C)]
    #[derive(Copy, Clone)]
    pub struct VkWriteDescriptorSet {
        sType: VkStructureType,
        pNext: *const c_void,
        dstSet: VkDescriptorSet,
        dstBinding: u32,
        dstArrayElement: u32,
        descriptorCount: u32,
        descriptorType: VkDescriptorType,
        pImageInfo: *const VkDescriptorImageInfo,
        pBufferInfo: *const VkDescriptorBufferInfo,
        pTexelBufferView: *const VkBufferView,
    }

    #[allow(non_snake_case)]
    #[repr(C)]
    #[derive(Copy, Clone)]
    pub struct VkBufferCreateInfo {
        sType: VkStructureType,
        pNext: *const c_void,
        flags: VkBufferCreateFlags,
        size: VkDeviceSize,
        usage: VkBufferUsageFlags,
        sharingMode: VkSharingMode,
        queueFamilyIndexCount: u32,
        pQueueFamilyIndices: *const u32,
    }

    #[allow(non_snake_case)]
    #[repr(C)]
    #[derive(Copy, Clone)]
    pub struct VkBufferViewCreateInfo {
        sType: VkStructureType,
        pNext: *const c_void,
        flags: VkBufferViewCreateFlags,
        buffer: VkBuffer,
        format: VkFormat,
        offset: VkDeviceSize,
        range: VkDeviceSize,
    }

    #[allow(non_snake_case)]
    #[repr(C)]
    #[derive(Copy, Clone)]
    pub struct VkImageSubresource {
        aspectMask: VkImageAspectFlags,
        mipLevel: u32,
        arrayLayer: u32,
    }

    #[allow(non_snake_case)]
    #[repr(C)]
    #[derive(Copy, Clone)]
    pub struct VkImageSubresourceLayers {
        aspectMask: VkImageAspectFlags,
        mipLevel: u32,
        baseArrayLayer: u32,
        layerCount: u32,
    }

    #[allow(non_snake_case)]
    #[repr(C)]
    #[derive(Copy, Clone)]
    pub struct VkImageSubresourceRange {
        aspectMask: VkImageAspectFlags,
        baseMipLevel: u32,
        levelCount: u32,
        baseArrayLayer: u32,
        layerCount: u32,
    }

    #[allow(non_snake_case)]
    #[repr(C)]
    #[derive(Copy, Clone)]
    pub struct VkMemoryBarrier {
        sType: VkStructureType,
        pNext: *const c_void,
        srcAccessMask: VkAccessFlags,
        dstAccessMask: VkAccessFlags,
    }

    #[allow(non_snake_case)]
    #[repr(C)]
    #[derive(Copy, Clone)]
    pub struct VkBufferMemoryBarrier {
        sType: VkStructureType,
        pNext: *const c_void,
        srcAccessMask: VkAccessFlags,
        dstAccessMask: VkAccessFlags,
        srcQueueFamilyIndex: u32,
        dstQueueFamilyIndex: u32,
        buffer: VkBuffer,
        offset: VkDeviceSize,
        size: VkDeviceSize,
    }

    #[allow(non_snake_case)]
    #[repr(C)]
    #[derive(Copy, Clone)]
    pub struct VkImageMemoryBarrier {
        sType: VkStructureType,
        pNext: *const c_void,
        srcAccessMask: VkAccessFlags,
        dstAccessMask: VkAccessFlags,
        oldLayout: VkImageLayout,
        newLayout: VkImageLayout,
        srcQueueFamilyIndex: u32,
        dstQueueFamilyIndex: u32,
        image: VkImage,
        subresourceRange: VkImageSubresourceRange,
    }

    #[allow(non_snake_case)]
    #[repr(C)]
    #[derive(Copy, Clone)]
    pub struct VkImageCreateInfo {
        sType: VkStructureType,
        pNext: *const c_void,
        flags: VkImageCreateFlags,
        imageType: VkImageType,
        format: VkFormat,
        extent: VkExtent3D,
        mipLevels: u32,
        arrayLayers: u32,
        samples: VkSampleCountFlagBits,
        tiling: VkImageTiling,
        usage: VkImageUsageFlags,
        sharingMode: VkSharingMode,
        queueFamilyIndexCount: u32,
        pQueueFamilyIndices: *const u32,
        initialLayout: VkImageLayout,
    }

    #[allow(non_snake_case)]
    #[repr(C)]
    #[derive(Copy, Clone)]
    pub struct VkSubresourceLayout {
        offset: VkDeviceSize,
        size: VkDeviceSize,
        rowPitch: VkDeviceSize,
        arrayPitch: VkDeviceSize,
        depthPitch: VkDeviceSize,
    }

    #[allow(non_snake_case)]
    #[repr(C)]
    #[derive(Copy, Clone)]
    pub struct VkImageViewCreateInfo {
        sType: VkStructureType,
        pNext: *const c_void,
        flags: VkImageViewCreateFlags,
        image: VkImage,
        viewType: VkImageViewType,
        format: VkFormat,
        components: VkComponentMapping,
        subresourceRange: VkImageSubresourceRange,
    }

    #[allow(non_snake_case)]
    #[repr(C)]
    #[derive(Copy, Clone)]
    pub struct VkBufferCopy {
        srcOffset: VkDeviceSize,
        dstOffset: VkDeviceSize,
        size: VkDeviceSize,
    }

    #[allow(non_snake_case)]
    #[repr(C)]
    #[derive(Copy, Clone)]
    pub struct VkSparseMemoryBind {
        resourceOffset: VkDeviceSize,
        size: VkDeviceSize,
        memory: VkDeviceMemory,
        memoryOffset: VkDeviceSize,
        flags: VkSparseMemoryBindFlags,
    }

    #[allow(non_snake_case)]
    #[repr(C)]
    #[derive(Copy, Clone)]
    pub struct VkSparseImageMemoryBind {
        subresource: VkImageSubresource,
        offset: VkOffset3D,
        extent: VkExtent3D,
        memory: VkDeviceMemory,
        memoryOffset: VkDeviceSize,
        flags: VkSparseMemoryBindFlags,
    }

    #[allow(non_snake_case)]
    #[repr(C)]
    #[derive(Copy, Clone)]
    pub struct VkSparseBufferMemoryBindInfo {
        buffer: VkBuffer,
        bindCount: u32,
        pBinds: *const VkSparseMemoryBind,
    }

    #[allow(non_snake_case)]
    #[repr(C)]
    #[derive(Copy, Clone)]
    pub struct VkSparseImageOpaqueMemoryBindInfo {
        image: VkImage,
        bindCount: u32,
        pBinds: *const VkSparseMemoryBind,
    }

    #[allow(non_snake_case)]
    #[repr(C)]
    #[derive(Copy, Clone)]
    pub struct VkSparseImageMemoryBindInfo {
        image: VkImage,
        bindCount: u32,
        pBinds: *const VkSparseImageMemoryBind,
    }

    #[allow(non_snake_case)]
    #[repr(C)]
    #[derive(Copy, Clone)]
    pub struct VkBindSparseInfo {
        sType: VkStructureType,
        pNext: *const c_void,
        waitSemaphoreCount: u32,
        pWaitSemaphores: *const VkSemaphore,
        bufferBindCount: u32,
        pBufferBinds: *const VkSparseBufferMemoryBindInfo,
        imageOpaqueBindCount: u32,
        pImageOpaqueBinds: *const VkSparseImageOpaqueMemoryBindInfo,
        imageBindCount: u32,
        pImageBinds: *const VkSparseImageMemoryBindInfo,
        signalSemaphoreCount: u32,
        pSignalSemaphores: *const VkSemaphore,
    }

    #[allow(non_snake_case)]
    #[repr(C)]
    #[derive(Copy, Clone)]
    pub struct VkImageCopy {
        srcSubresource: VkImageSubresourceLayers,
        srcOffset: VkOffset3D,
        dstSubresource: VkImageSubresourceLayers,
        dstOffset: VkOffset3D,
        extent: VkExtent3D,
    }

    #[allow(non_snake_case)]
    #[repr(C)]
    #[derive(Copy, Clone)]
    pub struct VkImageBlit {
        srcSubresource: VkImageSubresourceLayers,
        srcOffsets: [VkOffset3D; 2],
        dstSubresource: VkImageSubresourceLayers,
        dstOffsets: [VkOffset3D; 2],
    }

    #[allow(non_snake_case)]
    #[repr(C)]
    #[derive(Copy, Clone)]
    pub struct VkBufferImageCopy {
        bufferOffset: VkDeviceSize,
        bufferRowLength: u32,
        bufferImageHeight: u32,
        imageSubresource: VkImageSubresourceLayers,
        imageOffset: VkOffset3D,
        imageExtent: VkExtent3D,
    }

    #[allow(non_snake_case)]
    #[repr(C)]
    #[derive(Copy, Clone)]
    pub struct VkImageResolve {
        srcSubresource: VkImageSubresourceLayers,
        srcOffset: VkOffset3D,
        dstSubresource: VkImageSubresourceLayers,
        dstOffset: VkOffset3D,
        extent: VkExtent3D,
    }

    #[allow(non_snake_case)]
    #[repr(C)]
    #[derive(Copy, Clone)]
    pub struct VkShaderModuleCreateInfo {
        sType: VkStructureType,
        pNext: *const c_void,
        flags: VkShaderModuleCreateFlags,
        codeSize: usize,
        pCode: *const u32,
    }

    #[allow(non_snake_case)]
    #[repr(C)]
    #[derive(Copy, Clone)]
    pub struct VkDescriptorSetLayoutBinding {
        binding: u32,
        descriptorType: VkDescriptorType,
        descriptorCount: u32,
        stageFlags: VkShaderStageFlags,
        pImmutableSamplers: *const VkSampler,
    }

    #[allow(non_snake_case)]
    #[repr(C)]
    #[derive(Copy, Clone)]
    pub struct VkDescriptorSetLayoutCreateInfo {
        sType: VkStructureType,
        pNext: *const c_void,
        flags: VkDescriptorSetLayoutCreateFlags,
        bindingCount: u32,
        pBindings: *const VkDescriptorSetLayoutBinding,
    }

    #[allow(non_snake_case)]
    #[repr(C)]
    #[derive(Copy, Clone)]
    pub struct VkDescriptorPoolCreateInfo {
        sType: VkStructureType,
        pNext: *const c_void,
        flags: VkDescriptorPoolCreateFlags,
        maxSets: u32,
        poolSizeCount: u32,
        pPoolSizes: *const VkDescriptorPoolSize,
    }

    #[allow(non_snake_case)]
    #[repr(C)]
    #[derive(Copy, Clone)]
    pub struct VkPipelineShaderStageCreateInfo {
        sType: VkStructureType,
        pNext: *const c_void,
        flags: VkPipelineShaderStageCreateFlags,
        stage: VkShaderStageFlagBits,
        module: VkShaderModule,
        pName: *const c_char,
        pSpecializationInfo: *const VkSpecializationInfo,
    }

    #[allow(non_snake_case)]
    #[repr(C)]
    #[derive(Copy, Clone)]
    pub struct VkComputePipelineCreateInfo {
        sType: VkStructureType,
        pNext: *const c_void,
        flags: VkPipelineCreateFlags,
        stage: VkPipelineShaderStageCreateInfo,
        layout: VkPipelineLayout,
        basePipelineHandle: VkPipeline,
        basePipelineIndex: i32,
    }

    #[allow(non_snake_case)]
    #[repr(C)]
    #[derive(Copy, Clone)]
    pub struct VkPipelineVertexInputStateCreateInfo {
        sType: VkStructureType,
        pNext: *const c_void,
        flags: VkPipelineVertexInputStateCreateFlags,
        vertexBindingDescriptionCount: u32,
        pVertexBindingDescriptions: *const VkVertexInputBindingDescription,
        vertexAttributeDescriptionCount: u32,
        pVertexAttributeDescriptions: *const VkVertexInputAttributeDescription,
    }

    #[allow(non_snake_case)]
    #[repr(C)]
    #[derive(Copy, Clone)]
    pub struct VkPipelineInputAssemblyStateCreateInfo {
        sType: VkStructureType,
        pNext: *const c_void,
        flags: VkPipelineInputAssemblyStateCreateFlags,
        topology: VkPrimitiveTopology,
        primitiveRestartEnable: VkBool32,
    }

    #[allow(non_snake_case)]
    #[repr(C)]
    #[derive(Copy, Clone)]
    pub struct VkPipelineTessellationStateCreateInfo {
        sType: VkStructureType,
        pNext: *const c_void,
        flags: VkPipelineTessellationStateCreateFlags,
        patchControlPoints: u32,
    }

    #[allow(non_snake_case)]
    #[repr(C)]
    #[derive(Copy, Clone)]
    pub struct VkPipelineViewportStateCreateInfo {
        sType: VkStructureType,
        pNext: *const c_void,
        flags: VkPipelineViewportStateCreateFlags,
        viewportCount: u32,
        pViewports: *const VkViewport,
        scissorCount: u32,
        pScissors: *const VkRect2D,
    }

    #[allow(non_snake_case)]
    #[repr(C)]
    #[derive(Copy, Clone)]
    pub struct VkPipelineRasterizationStateCreateInfo {
        sType: VkStructureType,
        pNext: *const c_void,
        flags: VkPipelineRasterizationStateCreateFlags,
        depthClampEnable: VkBool32,
        rasterizerDiscardEnable: VkBool32,
        polygonMode: VkPolygonMode,
        cullMode: VkCullModeFlags,
        frontFace: VkFrontFace,
        depthBiasEnable: VkBool32,
        depthBiasConstantFactor: c_float,
        depthBiasClamp: c_float,
        depthBiasSlopeFactor: c_float,
        lineWidth: c_float,
    }

    #[allow(non_snake_case)]
    #[repr(C)]
    #[derive(Copy, Clone)]
    pub struct VkPipelineMultisampleStateCreateInfo {
        sType: VkStructureType,
        pNext: *const c_void,
        flags: VkPipelineMultisampleStateCreateFlags,
        rasterizationSamples: VkSampleCountFlagBits,
        sampleShadingEnable: VkBool32,
        minSampleShading: c_float,
        pSampleMask: *const VkSampleMask,
        alphaToCoverageEnable: VkBool32,
        alphaToOneEnable: VkBool32,
    }

    #[allow(non_snake_case)]
    #[repr(C)]
    #[derive(Copy, Clone)]
    pub struct VkPipelineColorBlendAttachmentState {
        blendEnable: VkBool32,
        srcColorBlendFactor: VkBlendFactor,
        dstColorBlendFactor: VkBlendFactor,
        colorBlendOp: VkBlendOp,
        srcAlphaBlendFactor: VkBlendFactor,
        dstAlphaBlendFactor: VkBlendFactor,
        alphaBlendOp: VkBlendOp,
        colorWriteMask: VkColorComponentFlags,
    }

    #[allow(non_snake_case)]
    #[repr(C)]
    #[derive(Copy, Clone)]
    pub struct VkPipelineColorBlendStateCreateInfo {
        sType: VkStructureType,
        pNext: *const c_void,
        flags: VkPipelineColorBlendStateCreateFlags,
        logicOpEnable: VkBool32,
        logicOp: VkLogicOp,
        attachmentCount: u32,
        pAttachments: *const VkPipelineColorBlendAttachmentState,
        blendConstants: [c_float; 4],
    }

    #[allow(non_snake_case)]
    #[repr(C)]
    #[derive(Copy, Clone)]
    pub struct VkPipelineDynamicStateCreateInfo {
        sType: VkStructureType,
        pNext: *const c_void,
        flags: VkPipelineDynamicStateCreateFlags,
        dynamicStateCount: u32,
        pDynamicStates: *const VkDynamicState,
    }

    #[allow(non_snake_case)]
    #[repr(C)]
    #[derive(Copy, Clone)]
    pub struct VkPipelineDepthStencilStateCreateInfo {
        sType: VkStructureType,
        pNext: *const c_void,
        flags: VkPipelineDepthStencilStateCreateFlags,
        depthTestEnable: VkBool32,
        depthWriteEnable: VkBool32,
        depthCompareOp: VkCompareOp,
        depthBoundsTestEnable: VkBool32,
        stencilTestEnable: VkBool32,
        front: VkStencilOpState,
        back: VkStencilOpState,
        minDepthBounds: c_float,
        maxDepthBounds: c_float,
    }

    #[allow(non_snake_case)]
    #[repr(C)]
    #[derive(Copy, Clone)]
    pub struct VkGraphicsPipelineCreateInfo {
        sType: VkStructureType,
        pNext: *const c_void,
        flags: VkPipelineCreateFlags,
        stageCount: u32,
        pStages: *const VkPipelineShaderStageCreateInfo,
        pVertexInputState: *const VkPipelineVertexInputStateCreateInfo,
        pInputAssemblyState: *const VkPipelineInputAssemblyStateCreateInfo,
        pTessellationState: *const VkPipelineTessellationStateCreateInfo,
        pViewportState: *const VkPipelineViewportStateCreateInfo,
        pRasterizationState: *const VkPipelineRasterizationStateCreateInfo,
        pMultisampleState: *const VkPipelineMultisampleStateCreateInfo,
        pDepthStencilState: *const VkPipelineDepthStencilStateCreateInfo,
        pColorBlendState: *const VkPipelineColorBlendStateCreateInfo,
        pDynamicState: *const VkPipelineDynamicStateCreateInfo,
        layout: VkPipelineLayout,
        renderPass: VkRenderPass,
        subpass: u32,
        basePipelineHandle: VkPipeline,
        basePipelineIndex: i32,
    }

    #[allow(non_snake_case)]
    #[repr(C)]
    #[derive(Copy, Clone)]
    pub struct VkPipelineCacheCreateInfo {
        sType: VkStructureType,
        pNext: *const c_void,
        flags: VkPipelineCacheCreateFlags,
        initialDataSize: usize,
        pInitialData: *const c_void,
    }

    #[allow(non_snake_case)]
    #[repr(C)]
    #[derive(Copy, Clone)]
    pub struct VkPushConstantRange {
        stageFlags: VkShaderStageFlags,
        offset: u32,
        size: u32,
    }

    #[allow(non_snake_case)]
    #[repr(C)]
    #[derive(Copy, Clone)]
    pub struct VkPipelineLayoutCreateInfo {
        sType: VkStructureType,
        pNext: *const c_void,
        flags: VkPipelineLayoutCreateFlags,
        setLayoutCount: u32,
        pSetLayouts: *const VkDescriptorSetLayout,
        pushConstantRangeCount: u32,
        pPushConstantRanges: *const VkPushConstantRange,
    }

    #[allow(non_snake_case)]
    #[repr(C)]
    #[derive(Copy, Clone)]
    pub struct VkSamplerCreateInfo {
        sType: VkStructureType,
        pNext: *const c_void,
        flags: VkSamplerCreateFlags,
        magFilter: VkFilter,
        minFilter: VkFilter,
        mipmapMode: VkSamplerMipmapMode,
        addressModeU: VkSamplerAddressMode,
        addressModeV: VkSamplerAddressMode,
        addressModeW: VkSamplerAddressMode,
        mipLodBias: c_float,
        anisotropyEnable: VkBool32,
        maxAnisotropy: c_float,
        compareEnable: VkBool32,
        compareOp: VkCompareOp,
        minLod: c_float,
        maxLod: c_float,
        borderColor: VkBorderColor,
        unnormalizedCoordinates: VkBool32,
    }

    #[allow(non_snake_case)]
    #[repr(C)]
    #[derive(Copy, Clone)]
    pub struct VkCommandPoolCreateInfo {
        sType: VkStructureType,
        pNext: *const c_void,
        flags: VkCommandPoolCreateFlags,
        queueFamilyIndex: u32,
    }

    #[allow(non_snake_case)]
    #[repr(C)]
    #[derive(Copy, Clone)]
    pub struct VkCommandBufferInheritanceInfo {
        sType: VkStructureType,
        pNext: *const c_void,
        renderPass: VkRenderPass,
        subpass: u32,
        framebuffer: VkFramebuffer,
        occlusionQueryEnable: VkBool32,
        queryFlags: VkQueryControlFlags,
        pipelineStatistics: VkQueryPipelineStatisticFlags,
    }

    #[allow(non_snake_case)]
    #[repr(C)]
    #[derive(Copy, Clone)]
    pub struct VkCommandBufferBeginInfo {
        sType: VkStructureType,
        pNext: *const c_void,
        flags: VkCommandBufferUsageFlags,
        pInheritanceInfo: *const VkCommandBufferInheritanceInfo,
    }

    #[allow(non_snake_case)]
    #[repr(C)]
    #[derive(Copy, Clone)]
    pub struct VkRenderPassBeginInfo {
        sType: VkStructureType,
        pNext: *const c_void,
        renderPass: VkRenderPass,
        framebuffer: VkFramebuffer,
        renderArea: VkRect2D,
        clearValueCount: u32,
        pClearValues: *const VkClearValue,
    }

    #[allow(non_snake_case)]
    #[repr(C)]
    #[derive(Copy, Clone)]
    pub struct VkClearAttachment {
        aspectMask: VkImageAspectFlags,
        colorAttachment: u32,
        clearValue: VkClearValue,
    }

    #[allow(non_snake_case)]
    #[repr(C)]
    #[derive(Copy, Clone)]
    pub struct VkAttachmentDescription {
        flags: VkAttachmentDescriptionFlags,
        format: VkFormat,
        samples: VkSampleCountFlagBits,
        loadOp: VkAttachmentLoadOp,
        storeOp: VkAttachmentStoreOp,
        stencilLoadOp: VkAttachmentLoadOp,
        stencilStoreOp: VkAttachmentStoreOp,
        initialLayout: VkImageLayout,
        finalLayout: VkImageLayout,
    }

    #[allow(non_snake_case)]
    #[repr(C)]
    #[derive(Copy, Clone)]
    pub struct VkSubpassDescription {
        flags: VkSubpassDescriptionFlags,
        pipelineBindPoint: VkPipelineBindPoint,
        inputAttachmentCount: u32,
        pInputAttachments: *const VkAttachmentReference,
        colorAttachmentCount: u32,
        pColorAttachments: *const VkAttachmentReference,
        pResolveAttachments: *const VkAttachmentReference,
        pDepthStencilAttachment: *const VkAttachmentReference,
        preserveAttachmentCount: u32,
        pPreserveAttachments: *const u32,
    }

    #[allow(non_snake_case)]
    #[repr(C)]
    #[derive(Copy, Clone)]
    pub struct VkSubpassDependency {
        srcSubpass: u32,
        dstSubpass: u32,
        srcStageMask: VkPipelineStageFlags,
        dstStageMask: VkPipelineStageFlags,
        srcAccessMask: VkAccessFlags,
        dstAccessMask: VkAccessFlags,
        dependencyFlags: VkDependencyFlags,
    }

    #[allow(non_snake_case)]
    #[repr(C)]
    #[derive(Copy, Clone)]
    pub struct VkRenderPassCreateInfo {
        sType: VkStructureType,
        pNext: *const c_void,
        flags: VkRenderPassCreateFlags,
        attachmentCount: u32,
        pAttachments: *const VkAttachmentDescription,
        subpassCount: u32,
        pSubpasses: *const VkSubpassDescription,
        dependencyCount: u32,
        pDependencies: *const VkSubpassDependency,
    }

    #[allow(non_snake_case)]
    #[repr(C)]
    #[derive(Copy, Clone)]
    pub struct VkEventCreateInfo {
        sType: VkStructureType,
        pNext: *const c_void,
        flags: VkEventCreateFlags,
    }

    #[allow(non_snake_case)]
    #[repr(C)]
    #[derive(Copy, Clone)]
    pub struct VkFenceCreateInfo {
        sType: VkStructureType,
        pNext: *const c_void,
        flags: VkFenceCreateFlags,
    }

    #[allow(non_snake_case)]
    #[repr(C)]
    #[derive(Copy, Clone)]
    pub struct VkPhysicalDeviceFeatures {
        robustBufferAccess: VkBool32,
        fullDrawIndexUint32: VkBool32,
        imageCubeArray: VkBool32,
        independentBlend: VkBool32,
        geometryShader: VkBool32,
        tessellationShader: VkBool32,
        sampleRateShading: VkBool32,
        dualSrcBlend: VkBool32,
        logicOp: VkBool32,
        multiDrawIndirect: VkBool32,
        drawIndirectFirstInstance: VkBool32,
        depthClamp: VkBool32,
        depthBiasClamp: VkBool32,
        fillModeNonSolid: VkBool32,
        depthBounds: VkBool32,
        wideLines: VkBool32,
        largePoints: VkBool32,
        alphaToOne: VkBool32,
        multiViewport: VkBool32,
        samplerAnisotropy: VkBool32,
        textureCompressionETC2: VkBool32,
        textureCompressionASTC_LDR: VkBool32,
        textureCompressionBC: VkBool32,
        occlusionQueryPrecise: VkBool32,
        pipelineStatisticsQuery: VkBool32,
        vertexPipelineStoresAndAtomics: VkBool32,
        fragmentStoresAndAtomics: VkBool32,
        shaderTessellationAndGeometryPointSize: VkBool32,
        shaderImageGatherExtended: VkBool32,
        shaderStorageImageExtendedFormats: VkBool32,
        shaderStorageImageMultisample: VkBool32,
        shaderStorageImageReadWithoutFormat: VkBool32,
        shaderStorageImageWriteWithoutFormat: VkBool32,
        shaderUniformBufferArrayDynamicIndexing: VkBool32,
        shaderSampledImageArrayDynamicIndexing: VkBool32,
        shaderStorageBufferArrayDynamicIndexing: VkBool32,
        shaderStorageImageArrayDynamicIndexing: VkBool32,
        shaderClipDistance: VkBool32,
        shaderCullDistance: VkBool32,
        shaderFloat64: VkBool32,
        shaderInt64: VkBool32,
        shaderInt16: VkBool32,
        shaderResourceResidency: VkBool32,
        shaderResourceMinLod: VkBool32,
        sparseBinding: VkBool32,
        sparseResidencyBuffer: VkBool32,
        sparseResidencyImage2D: VkBool32,
        sparseResidencyImage3D: VkBool32,
        sparseResidency2Samples: VkBool32,
        sparseResidency4Samples: VkBool32,
        sparseResidency8Samples: VkBool32,
        sparseResidency16Samples: VkBool32,
        sparseResidencyAliased: VkBool32,
        variableMultisampleRate: VkBool32,
        inheritedQueries: VkBool32,
    }

    #[allow(non_snake_case)]
    #[repr(C)]
    #[derive(Copy, Clone)]
    pub struct VkPhysicalDeviceSparseProperties {
        residencyStandard2DBlockShape: VkBool32,
        residencyStandard2DMultisampleBlockShape: VkBool32,
        residencyStandard3DBlockShape: VkBool32,
        residencyAlignedMipSize: VkBool32,
        residencyNonResidentStrict: VkBool32,
    }

    #[allow(non_snake_case)]
    #[repr(C)]
    #[derive(Copy, Clone)]
    pub struct VkPhysicalDeviceLimits {
        maxImageDimension1D: u32,
        maxImageDimension2D: u32,
        maxImageDimension3D: u32,
        maxImageDimensionCube: u32,
        maxImageArrayLayers: u32,
        maxTexelBufferElements: u32,
        maxUniformBufferRange: u32,
        maxStorageBufferRange: u32,
        maxPushConstantsSize: u32,
        maxMemoryAllocationCount: u32,
        maxSamplerAllocationCount: u32,
        bufferImageGranularity: VkDeviceSize,
        sparseAddressSpaceSize: VkDeviceSize,
        maxBoundDescriptorSets: u32,
        maxPerStageDescriptorSamplers: u32,
        maxPerStageDescriptorUniformBuffers: u32,
        maxPerStageDescriptorStorageBuffers: u32,
        maxPerStageDescriptorSampledImages: u32,
        maxPerStageDescriptorStorageImages: u32,
        maxPerStageDescriptorInputAttachments: u32,
        maxPerStageResources: u32,
        maxDescriptorSetSamplers: u32,
        maxDescriptorSetUniformBuffers: u32,
        maxDescriptorSetUniformBuffersDynamic: u32,
        maxDescriptorSetStorageBuffers: u32,
        maxDescriptorSetStorageBuffersDynamic: u32,
        maxDescriptorSetSampledImages: u32,
        maxDescriptorSetStorageImages: u32,
        maxDescriptorSetInputAttachments: u32,
        maxVertexInputAttributes: u32,
        maxVertexInputBindings: u32,
        maxVertexInputAttributeOffset: u32,
        maxVertexInputBindingStride: u32,
        maxVertexOutputComponents: u32,
        maxTessellationGenerationLevel: u32,
        maxTessellationPatchSize: u32,
        maxTessellationControlPerVertexInputComponents: u32,
        maxTessellationControlPerVertexOutputComponents: u32,
        maxTessellationControlPerPatchOutputComponents: u32,
        maxTessellationControlTotalOutputComponents: u32,
        maxTessellationEvaluationInputComponents: u32,
        maxTessellationEvaluationOutputComponents: u32,
        maxGeometryShaderInvocations: u32,
        maxGeometryInputComponents: u32,
        maxGeometryOutputComponents: u32,
        maxGeometryOutputVertices: u32,
        maxGeometryTotalOutputComponents: u32,
        maxFragmentInputComponents: u32,
        maxFragmentOutputAttachments: u32,
        maxFragmentDualSrcAttachments: u32,
        maxFragmentCombinedOutputResources: u32,
        maxComputeSharedMemorySize: u32,
        maxComputeWorkGroupCount: [u32; 3],
        maxComputeWorkGroupInvocations: u32,
        maxComputeWorkGroupSize: [u32; 3],
        subPixelPrecisionBits: u32,
        subTexelPrecisionBits: u32,
        mipmapPrecisionBits: u32,
        maxDrawIndexedIndexValue: u32,
        maxDrawIndirectCount: u32,
        maxSamplerLodBias: c_float,
        maxSamplerAnisotropy: c_float,
        maxViewports: u32,
        maxViewportDimensions: [u32; 2],
        viewportBoundsRange: [c_float; 2],
        viewportSubPixelBits: u32,
        minMemoryMapAlignment: usize,
        minTexelBufferOffsetAlignment: VkDeviceSize,
        minUniformBufferOffsetAlignment: VkDeviceSize,
        minStorageBufferOffsetAlignment: VkDeviceSize,
        minTexelOffset: i32,
        maxTexelOffset: u32,
        minTexelGatherOffset: i32,
        maxTexelGatherOffset: u32,
        minInterpolationOffset: c_float,
        maxInterpolationOffset: c_float,
        subPixelInterpolationOffsetBits: u32,
        maxFramebufferWidth: u32,
        maxFramebufferHeight: u32,
        maxFramebufferLayers: u32,
        framebufferColorSampleCounts: VkSampleCountFlags,
        framebufferDepthSampleCounts: VkSampleCountFlags,
        framebufferStencilSampleCounts: VkSampleCountFlags,
        framebufferNoAttachmentsSampleCounts: VkSampleCountFlags,
        maxColorAttachments: u32,
        sampledImageColorSampleCounts: VkSampleCountFlags,
        sampledImageIntegerSampleCounts: VkSampleCountFlags,
        sampledImageDepthSampleCounts: VkSampleCountFlags,
        sampledImageStencilSampleCounts: VkSampleCountFlags,
        storageImageSampleCounts: VkSampleCountFlags,
        maxSampleMaskWords: u32,
        timestampComputeAndGraphics: VkBool32,
        timestampPeriod: c_float,
        maxClipDistances: u32,
        maxCullDistances: u32,
        maxCombinedClipAndCullDistances: u32,
        discreteQueuePriorities: u32,
        pointSizeRange: [c_float; 2],
        lineWidthRange: [c_float; 2],
        pointSizeGranularity: c_float,
        lineWidthGranularity: c_float,
        strictLines: VkBool32,
        standardSampleLocations: VkBool32,
        optimalBufferCopyOffsetAlignment: VkDeviceSize,
        optimalBufferCopyRowPitchAlignment: VkDeviceSize,
        nonCoherentAtomSize: VkDeviceSize,
    }

    #[allow(non_snake_case)]
    #[repr(C)]
    #[derive(Copy, Clone)]
    pub struct VkSemaphoreCreateInfo {
        sType: VkStructureType,
        pNext: *const c_void,
        flags: VkSemaphoreCreateFlags,
    }

    #[allow(non_snake_case)]
    #[repr(C)]
    #[derive(Copy, Clone)]
    pub struct VkQueryPoolCreateInfo {
        sType: VkStructureType,
        pNext: *const c_void,
        flags: VkQueryPoolCreateFlags,
        queryType: VkQueryType,
        queryCount: u32,
        pipelineStatistics: VkQueryPipelineStatisticFlags,
    }

    #[allow(non_snake_case)]
    #[repr(C)]
    #[derive(Copy, Clone)]
    pub struct VkFramebufferCreateInfo {
        sType: VkStructureType,
        pNext: *const c_void,
        flags: VkFramebufferCreateFlags,
        renderPass: VkRenderPass,
        attachmentCount: u32,
        pAttachments: *const VkImageView,
        width: u32,
        height: u32,
        layers: u32,
    }

    #[allow(non_snake_case)]
    #[repr(C)]
    #[derive(Copy, Clone)]
    pub struct VkSubmitInfo {
        sType: VkStructureType,
        pNext: *const c_void,
        waitSemaphoreCount: u32,
        pWaitSemaphores: *const VkSemaphore,
        pWaitDstStageMask: *const VkPipelineStageFlags,
        commandBufferCount: u32,
        pCommandBuffers: *const VkCommandBuffer,
        signalSemaphoreCount: u32,
        pSignalSemaphores: *const VkSemaphore,
    }

    #[allow(non_snake_case)]
    #[repr(C)]
    #[derive(Copy, Clone)]
    pub struct VkExternalMemoryProperties {
        externalMemoryFeatures: VkExternalMemoryFeatureFlags,
        exportFromImportedHandleTypes: VkExternalMemoryHandleTypeFlags,
        compatibleHandleTypes: VkExternalMemoryHandleTypeFlags,
    }
    pub type VkExternalMemoryPropertiesKHR = VkExternalMemoryProperties;

    #[allow(non_snake_case)]
    #[repr(C)]
    #[derive(Copy, Clone)]
    pub struct VkExternalImageFormatProperties {
        sType: VkStructureType,
        pNext: *mut c_void,
        externalMemoryProperties: VkExternalMemoryProperties,
    }
    pub type VkExternalImageFormatPropertiesKHR = VkExternalImageFormatProperties;

    #[allow(non_snake_case)]
    #[repr(C)]
    #[derive(Copy, Clone)]
    pub struct VkPhysicalDeviceExternalBufferInfo {
        sType: VkStructureType,
        pNext: *const c_void,
        flags: VkBufferCreateFlags,
        usage: VkBufferUsageFlags,
        handleType: VkExternalMemoryHandleTypeFlagBits,
    }
    pub type VkPhysicalDeviceExternalBufferInfoKHR = VkPhysicalDeviceExternalBufferInfo;

    #[allow(non_snake_case)]
    #[repr(C)]
    #[derive(Copy, Clone)]
    pub struct VkExternalBufferProperties {
        sType: VkStructureType,
        pNext: *mut c_void,
        externalMemoryProperties: VkExternalMemoryProperties,
    }
    pub type VkExternalBufferPropertiesKHR = VkExternalBufferProperties;

    #[allow(non_snake_case)]
    #[repr(C)]
    #[derive(Copy, Clone)]
    pub struct VkPhysicalDeviceIDProperties {
        sType: VkStructureType,
        pNext: *mut c_void,
        deviceUUID: u8,
        driverUUID: u8,
        deviceLUID: u8,
        deviceNodeMask: u32,
        deviceLUIDValid: VkBool32,
    }
    pub type VkPhysicalDeviceIDPropertiesKHR = VkPhysicalDeviceIDProperties;

    #[allow(non_snake_case)]
    #[repr(C)]
    #[derive(Copy, Clone)]
    pub struct VkExternalMemoryImageCreateInfo {
        sType: VkStructureType,
        pNext: *const c_void,
        handleTypes: VkExternalMemoryHandleTypeFlags,
    }
    pub type VkExternalMemoryImageCreateInfoKHR = VkExternalMemoryImageCreateInfo;

    #[allow(non_snake_case)]
    #[repr(C)]
    #[derive(Copy, Clone)]
    pub struct VkExternalMemoryBufferCreateInfo {
        sType: VkStructureType,
        pNext: *const c_void,
        handleTypes: VkExternalMemoryHandleTypeFlags,
    }
    pub type VkExternalMemoryBufferCreateInfoKHR = VkExternalMemoryBufferCreateInfo;

    #[allow(non_snake_case)]
    #[repr(C)]
    #[derive(Copy, Clone)]
    pub struct VkExportMemoryAllocateInfo {
        sType: VkStructureType,
        pNext: *const c_void,
        handleTypes: VkExternalMemoryHandleTypeFlags,
    }
    pub type VkExportMemoryAllocateInfoKHR = VkExportMemoryAllocateInfo;

    #[allow(non_snake_case)]
    #[repr(C)]
    #[derive(Copy, Clone)]
    pub struct VkPhysicalDeviceProperties {
        apiVersion: u32,
        driverVersion: u32,
        vendorID: u32,
        deviceID: u32,
        deviceType: VkPhysicalDeviceType,
        deviceName: c_char,
        pipelineCacheUUID: u8,
        limits: VkPhysicalDeviceLimits,
        sparseProperties: VkPhysicalDeviceSparseProperties,
    }

    #[allow(non_snake_case)]
    #[repr(C)]
    #[derive(Copy, Clone)]
    pub struct VkDeviceCreateInfo {
        sType: VkStructureType,
        pNext: *const c_void,
        flags: VkDeviceCreateFlags,
        queueCreateInfoCount: u32,
        pQueueCreateInfos: *const VkDeviceQueueCreateInfo,
        enabledLayerCount: u32,
        ppEnabledLayerNames: *const *const c_char,
        enabledExtensionCount: u32,
        ppEnabledExtensionNames: *const *const c_char,
        pEnabledFeatures: *const VkPhysicalDeviceFeatures,
    }

    #[allow(non_snake_case)]
    #[repr(C)]
    #[derive(Copy, Clone)]
    pub struct VkPhysicalDeviceMemoryProperties {
        memoryTypeCount: u32,
        memoryTypes: VkMemoryType,
        memoryHeapCount: u32,
        memoryHeaps: VkMemoryHeap,
    }
}

pub mod enumerations {
    #![allow(dead_code, non_upper_case_globals, unused_imports)]

    use super::types::*;
    use std::os::raw::*;

    pub const ATTACHMENT_UNUSED: c_uint = !0;
    pub const FALSE: c_uint = 0;
    pub const KHR_EXTERNAL_MEMORY_CAPABILITIES_EXTENSION_NAME: &str =
        "VK_KHR_external_memory_capabilities\0";
    pub const KHR_EXTERNAL_MEMORY_CAPABILITIES_SPEC_VERSION: c_uint = 1;
    pub const KHR_EXTERNAL_MEMORY_EXTENSION_NAME: &str = "VK_KHR_external_memory\0";
    pub const KHR_EXTERNAL_MEMORY_SPEC_VERSION: c_uint = 1;
    pub const KHR_EXTERNAL_MEMORY_WIN32_EXTENSION_NAME: &str = "VK_KHR_external_memory_win32\0";
    pub const KHR_EXTERNAL_MEMORY_WIN32_SPEC_VERSION: c_uint = 1;
    pub const LOD_CLAMP_NONE: c_float = 1000.0;
    pub const LUID_SIZE: c_uint = 8;
    pub const LUID_SIZE_KHR: c_uint = 8;
    pub const MAX_DESCRIPTION_SIZE: c_uint = 256;
    pub const MAX_EXTENSION_NAME_SIZE: c_uint = 256;
    pub const MAX_MEMORY_HEAPS: c_uint = 16;
    pub const MAX_MEMORY_TYPES: c_uint = 32;
    pub const MAX_PHYSICAL_DEVICE_NAME_SIZE: c_uint = 256;
    pub const QUEUE_FAMILY_EXTERNAL: c_uint = !1;
    pub const QUEUE_FAMILY_EXTERNAL_KHR: c_uint = !1;
    pub const QUEUE_FAMILY_IGNORED: c_uint = !0;
    pub const REMAINING_ARRAY_LAYERS: c_uint = !0;
    pub const REMAINING_MIP_LEVELS: c_uint = !0;
    pub const SUBPASS_EXTERNAL: c_uint = !0;
    pub const TRUE: c_uint = 1;
    pub const UUID_SIZE: c_uint = 16;
    pub const WHOLE_SIZE: c_uint = !0;
}

pub mod functions {
    #![allow(non_snake_case, unused_variables, dead_code, unused_imports)]

    use super::types::*;
    use super::*;
    use std::mem::transmute;
    use std::os::raw::*;

    macro_rules! func {
        ($fun:ident, $ret:ty, $($name:ident: $typ:ty),*) => {
            #[allow(non_snake_case)]
            #[inline] pub unsafe fn $fun($($name: $typ),*) -> $ret {
                transmute::<_, extern "system" fn($($typ),*) -> $ret>(storage::$fun.ptr.clone())($($name),*)
            }
        }
    }

    func!(AllocateCommandBuffers, VkResult, device: VkDevice, pAllocateInfo: *const VkCommandBufferAllocateInfo, pCommandBuffers: *mut VkCommandBuffer);
    func!(AllocateDescriptorSets, VkResult, device: VkDevice, pAllocateInfo: *const VkDescriptorSetAllocateInfo, pDescriptorSets: *mut VkDescriptorSet);
    func!(AllocateMemory, VkResult, device: VkDevice, pAllocateInfo: *const VkMemoryAllocateInfo, pAllocator: *const VkAllocationCallbacks, pMemory: *mut VkDeviceMemory);
    func!(BeginCommandBuffer, VkResult, commandBuffer: VkCommandBuffer, pBeginInfo: *const VkCommandBufferBeginInfo);
    func!(BindBufferMemory, VkResult, device: VkDevice, buffer: VkBuffer, memory: VkDeviceMemory, memoryOffset: VkDeviceSize);
    func!(BindImageMemory, VkResult, device: VkDevice, image: VkImage, memory: VkDeviceMemory, memoryOffset: VkDeviceSize);
    func!(CmdBeginQuery, (), commandBuffer: VkCommandBuffer, queryPool: VkQueryPool, query: u32, flags: VkQueryControlFlags);
    func!(CmdBeginRenderPass, (), commandBuffer: VkCommandBuffer, pRenderPassBegin: *const VkRenderPassBeginInfo, contents: VkSubpassContents);
    func!(CmdBindDescriptorSets, (), commandBuffer: VkCommandBuffer, pipelineBindPoint: VkPipelineBindPoint, layout: VkPipelineLayout, firstSet: u32, descriptorSetCount: u32, pDescriptorSets: *const VkDescriptorSet, dynamicOffsetCount: u32, pDynamicOffsets: *const u32);
    func!(CmdBindIndexBuffer, (), commandBuffer: VkCommandBuffer, buffer: VkBuffer, offset: VkDeviceSize, indexType: VkIndexType);
    func!(CmdBindPipeline, (), commandBuffer: VkCommandBuffer, pipelineBindPoint: VkPipelineBindPoint, pipeline: VkPipeline);
    func!(CmdBindVertexBuffers, (), commandBuffer: VkCommandBuffer, firstBinding: u32, bindingCount: u32, pBuffers: *const VkBuffer, pOffsets: *const VkDeviceSize);
    func!(CmdBlitImage, (), commandBuffer: VkCommandBuffer, srcImage: VkImage, srcImageLayout: VkImageLayout, dstImage: VkImage, dstImageLayout: VkImageLayout, regionCount: u32, pRegions: *const VkImageBlit, filter: VkFilter);
    func!(CmdClearAttachments, (), commandBuffer: VkCommandBuffer, attachmentCount: u32, pAttachments: *const VkClearAttachment, rectCount: u32, pRects: *const VkClearRect);
    func!(CmdClearColorImage, (), commandBuffer: VkCommandBuffer, image: VkImage, imageLayout: VkImageLayout, pColor: *const VkClearColorValue, rangeCount: u32, pRanges: *const VkImageSubresourceRange);
    func!(CmdClearDepthStencilImage, (), commandBuffer: VkCommandBuffer, image: VkImage, imageLayout: VkImageLayout, pDepthStencil: *const VkClearDepthStencilValue, rangeCount: u32, pRanges: *const VkImageSubresourceRange);
    func!(CmdCopyBuffer, (), commandBuffer: VkCommandBuffer, srcBuffer: VkBuffer, dstBuffer: VkBuffer, regionCount: u32, pRegions: *const VkBufferCopy);
    func!(CmdCopyBufferToImage, (), commandBuffer: VkCommandBuffer, srcBuffer: VkBuffer, dstImage: VkImage, dstImageLayout: VkImageLayout, regionCount: u32, pRegions: *const VkBufferImageCopy);
    func!(CmdCopyImage, (), commandBuffer: VkCommandBuffer, srcImage: VkImage, srcImageLayout: VkImageLayout, dstImage: VkImage, dstImageLayout: VkImageLayout, regionCount: u32, pRegions: *const VkImageCopy);
    func!(CmdCopyImageToBuffer, (), commandBuffer: VkCommandBuffer, srcImage: VkImage, srcImageLayout: VkImageLayout, dstBuffer: VkBuffer, regionCount: u32, pRegions: *const VkBufferImageCopy);
    func!(CmdCopyQueryPoolResults, (), commandBuffer: VkCommandBuffer, queryPool: VkQueryPool, firstQuery: u32, queryCount: u32, dstBuffer: VkBuffer, dstOffset: VkDeviceSize, stride: VkDeviceSize, flags: VkQueryResultFlags);
    func!(CmdDispatch, (), commandBuffer: VkCommandBuffer, groupCountX: u32, groupCountY: u32, groupCountZ: u32);
    func!(CmdDispatchIndirect, (), commandBuffer: VkCommandBuffer, buffer: VkBuffer, offset: VkDeviceSize);
    func!(CmdDraw, (), commandBuffer: VkCommandBuffer, vertexCount: u32, instanceCount: u32, firstVertex: u32, firstInstance: u32);
    func!(CmdDrawIndexed, (), commandBuffer: VkCommandBuffer, indexCount: u32, instanceCount: u32, firstIndex: u32, vertexOffset: i32, firstInstance: u32);
    func!(CmdDrawIndexedIndirect, (), commandBuffer: VkCommandBuffer, buffer: VkBuffer, offset: VkDeviceSize, drawCount: u32, stride: u32);
    func!(CmdDrawIndirect, (), commandBuffer: VkCommandBuffer, buffer: VkBuffer, offset: VkDeviceSize, drawCount: u32, stride: u32);
    func!(CmdEndQuery, (), commandBuffer: VkCommandBuffer, queryPool: VkQueryPool, query: u32);
    func!(CmdEndRenderPass, (), commandBuffer: VkCommandBuffer);
    func!(CmdExecuteCommands, (), commandBuffer: VkCommandBuffer, commandBufferCount: u32, pCommandBuffers: *const VkCommandBuffer);
    func!(CmdFillBuffer, (), commandBuffer: VkCommandBuffer, dstBuffer: VkBuffer, dstOffset: VkDeviceSize, size: VkDeviceSize, data: u32);
    func!(CmdNextSubpass, (), commandBuffer: VkCommandBuffer, contents: VkSubpassContents);
    func!(CmdPipelineBarrier, (), commandBuffer: VkCommandBuffer, srcStageMask: VkPipelineStageFlags, dstStageMask: VkPipelineStageFlags, dependencyFlags: VkDependencyFlags, memoryBarrierCount: u32, pMemoryBarriers: *const VkMemoryBarrier, bufferMemoryBarrierCount: u32, pBufferMemoryBarriers: *const VkBufferMemoryBarrier, imageMemoryBarrierCount: u32, pImageMemoryBarriers: *const VkImageMemoryBarrier);
    func!(CmdPushConstants, (), commandBuffer: VkCommandBuffer, layout: VkPipelineLayout, stageFlags: VkShaderStageFlags, offset: u32, size: u32, pValues: *const c_void);
    func!(CmdResetEvent, (), commandBuffer: VkCommandBuffer, event: VkEvent, stageMask: VkPipelineStageFlags);
    func!(CmdResetQueryPool, (), commandBuffer: VkCommandBuffer, queryPool: VkQueryPool, firstQuery: u32, queryCount: u32);
    func!(CmdResolveImage, (), commandBuffer: VkCommandBuffer, srcImage: VkImage, srcImageLayout: VkImageLayout, dstImage: VkImage, dstImageLayout: VkImageLayout, regionCount: u32, pRegions: *const VkImageResolve);
    func!(CmdSetBlendConstants, (), commandBuffer: VkCommandBuffer, blendConstants: [c_float;4]);
    func!(CmdSetDepthBias, (), commandBuffer: VkCommandBuffer, depthBiasConstantFactor: c_float, depthBiasClamp: c_float, depthBiasSlopeFactor: c_float);
    func!(CmdSetDepthBounds, (), commandBuffer: VkCommandBuffer, minDepthBounds: c_float, maxDepthBounds: c_float);
    func!(CmdSetEvent, (), commandBuffer: VkCommandBuffer, event: VkEvent, stageMask: VkPipelineStageFlags);
    func!(CmdSetLineWidth, (), commandBuffer: VkCommandBuffer, lineWidth: c_float);
    func!(CmdSetScissor, (), commandBuffer: VkCommandBuffer, firstScissor: u32, scissorCount: u32, pScissors: *const VkRect2D);
    func!(CmdSetStencilCompareMask, (), commandBuffer: VkCommandBuffer, faceMask: VkStencilFaceFlags, compareMask: u32);
    func!(CmdSetStencilReference, (), commandBuffer: VkCommandBuffer, faceMask: VkStencilFaceFlags, reference: u32);
    func!(CmdSetStencilWriteMask, (), commandBuffer: VkCommandBuffer, faceMask: VkStencilFaceFlags, writeMask: u32);
    func!(CmdSetViewport, (), commandBuffer: VkCommandBuffer, firstViewport: u32, viewportCount: u32, pViewports: *const VkViewport);
    func!(CmdUpdateBuffer, (), commandBuffer: VkCommandBuffer, dstBuffer: VkBuffer, dstOffset: VkDeviceSize, dataSize: VkDeviceSize, pData: *const c_void);
    func!(CmdWaitEvents, (), commandBuffer: VkCommandBuffer, eventCount: u32, pEvents: *const VkEvent, srcStageMask: VkPipelineStageFlags, dstStageMask: VkPipelineStageFlags, memoryBarrierCount: u32, pMemoryBarriers: *const VkMemoryBarrier, bufferMemoryBarrierCount: u32, pBufferMemoryBarriers: *const VkBufferMemoryBarrier, imageMemoryBarrierCount: u32, pImageMemoryBarriers: *const VkImageMemoryBarrier);
    func!(CmdWriteTimestamp, (), commandBuffer: VkCommandBuffer, pipelineStage: VkPipelineStageFlagBits, queryPool: VkQueryPool, query: u32);
    func!(CreateBuffer, VkResult, device: VkDevice, pCreateInfo: *const VkBufferCreateInfo, pAllocator: *const VkAllocationCallbacks, pBuffer: *mut VkBuffer);
    func!(CreateBufferView, VkResult, device: VkDevice, pCreateInfo: *const VkBufferViewCreateInfo, pAllocator: *const VkAllocationCallbacks, pView: *mut VkBufferView);
    func!(CreateCommandPool, VkResult, device: VkDevice, pCreateInfo: *const VkCommandPoolCreateInfo, pAllocator: *const VkAllocationCallbacks, pCommandPool: *mut VkCommandPool);
    func!(CreateComputePipelines, VkResult, device: VkDevice, pipelineCache: VkPipelineCache, createInfoCount: u32, pCreateInfos: *const VkComputePipelineCreateInfo, pAllocator: *const VkAllocationCallbacks, pPipelines: *mut VkPipeline);
    func!(CreateDescriptorPool, VkResult, device: VkDevice, pCreateInfo: *const VkDescriptorPoolCreateInfo, pAllocator: *const VkAllocationCallbacks, pDescriptorPool: *mut VkDescriptorPool);
    func!(CreateDescriptorSetLayout, VkResult, device: VkDevice, pCreateInfo: *const VkDescriptorSetLayoutCreateInfo, pAllocator: *const VkAllocationCallbacks, pSetLayout: *mut VkDescriptorSetLayout);
    func!(CreateDevice, VkResult, physicalDevice: VkPhysicalDevice, pCreateInfo: *const VkDeviceCreateInfo, pAllocator: *const VkAllocationCallbacks, pDevice: *mut VkDevice);
    func!(CreateEvent, VkResult, device: VkDevice, pCreateInfo: *const VkEventCreateInfo, pAllocator: *const VkAllocationCallbacks, pEvent: *mut VkEvent);
    func!(CreateFence, VkResult, device: VkDevice, pCreateInfo: *const VkFenceCreateInfo, pAllocator: *const VkAllocationCallbacks, pFence: *mut VkFence);
    func!(CreateFramebuffer, VkResult, device: VkDevice, pCreateInfo: *const VkFramebufferCreateInfo, pAllocator: *const VkAllocationCallbacks, pFramebuffer: *mut VkFramebuffer);
    func!(CreateGraphicsPipelines, VkResult, device: VkDevice, pipelineCache: VkPipelineCache, createInfoCount: u32, pCreateInfos: *const VkGraphicsPipelineCreateInfo, pAllocator: *const VkAllocationCallbacks, pPipelines: *mut VkPipeline);
    func!(CreateImage, VkResult, device: VkDevice, pCreateInfo: *const VkImageCreateInfo, pAllocator: *const VkAllocationCallbacks, pImage: *mut VkImage);
    func!(CreateImageView, VkResult, device: VkDevice, pCreateInfo: *const VkImageViewCreateInfo, pAllocator: *const VkAllocationCallbacks, pView: *mut VkImageView);
    func!(CreateInstance, VkResult, pCreateInfo: *const VkInstanceCreateInfo, pAllocator: *const VkAllocationCallbacks, pInstance: *mut VkInstance);
    func!(CreatePipelineCache, VkResult, device: VkDevice, pCreateInfo: *const VkPipelineCacheCreateInfo, pAllocator: *const VkAllocationCallbacks, pPipelineCache: *mut VkPipelineCache);
    func!(CreatePipelineLayout, VkResult, device: VkDevice, pCreateInfo: *const VkPipelineLayoutCreateInfo, pAllocator: *const VkAllocationCallbacks, pPipelineLayout: *mut VkPipelineLayout);
    func!(CreateQueryPool, VkResult, device: VkDevice, pCreateInfo: *const VkQueryPoolCreateInfo, pAllocator: *const VkAllocationCallbacks, pQueryPool: *mut VkQueryPool);
    func!(CreateRenderPass, VkResult, device: VkDevice, pCreateInfo: *const VkRenderPassCreateInfo, pAllocator: *const VkAllocationCallbacks, pRenderPass: *mut VkRenderPass);
    func!(CreateSampler, VkResult, device: VkDevice, pCreateInfo: *const VkSamplerCreateInfo, pAllocator: *const VkAllocationCallbacks, pSampler: *mut VkSampler);
    func!(CreateSemaphore, VkResult, device: VkDevice, pCreateInfo: *const VkSemaphoreCreateInfo, pAllocator: *const VkAllocationCallbacks, pSemaphore: *mut VkSemaphore);
    func!(CreateShaderModule, VkResult, device: VkDevice, pCreateInfo: *const VkShaderModuleCreateInfo, pAllocator: *const VkAllocationCallbacks, pShaderModule: *mut VkShaderModule);
    func!(DestroyBuffer, (), device: VkDevice, buffer: VkBuffer, pAllocator: *const VkAllocationCallbacks);
    func!(DestroyBufferView, (), device: VkDevice, bufferView: VkBufferView, pAllocator: *const VkAllocationCallbacks);
    func!(DestroyCommandPool, (), device: VkDevice, commandPool: VkCommandPool, pAllocator: *const VkAllocationCallbacks);
    func!(DestroyDescriptorPool, (), device: VkDevice, descriptorPool: VkDescriptorPool, pAllocator: *const VkAllocationCallbacks);
    func!(DestroyDescriptorSetLayout, (), device: VkDevice, descriptorSetLayout: VkDescriptorSetLayout, pAllocator: *const VkAllocationCallbacks);
    func!(DestroyDevice, (), device: VkDevice, pAllocator: *const VkAllocationCallbacks);
    func!(DestroyEvent, (), device: VkDevice, event: VkEvent, pAllocator: *const VkAllocationCallbacks);
    func!(DestroyFence, (), device: VkDevice, fence: VkFence, pAllocator: *const VkAllocationCallbacks);
    func!(DestroyFramebuffer, (), device: VkDevice, framebuffer: VkFramebuffer, pAllocator: *const VkAllocationCallbacks);
    func!(DestroyImage, (), device: VkDevice, image: VkImage, pAllocator: *const VkAllocationCallbacks);
    func!(DestroyImageView, (), device: VkDevice, imageView: VkImageView, pAllocator: *const VkAllocationCallbacks);
    func!(DestroyInstance, (), instance: VkInstance, pAllocator: *const VkAllocationCallbacks);
    func!(DestroyPipeline, (), device: VkDevice, pipeline: VkPipeline, pAllocator: *const VkAllocationCallbacks);
    func!(DestroyPipelineCache, (), device: VkDevice, pipelineCache: VkPipelineCache, pAllocator: *const VkAllocationCallbacks);
    func!(DestroyPipelineLayout, (), device: VkDevice, pipelineLayout: VkPipelineLayout, pAllocator: *const VkAllocationCallbacks);
    func!(DestroyQueryPool, (), device: VkDevice, queryPool: VkQueryPool, pAllocator: *const VkAllocationCallbacks);
    func!(DestroyRenderPass, (), device: VkDevice, renderPass: VkRenderPass, pAllocator: *const VkAllocationCallbacks);
    func!(DestroySampler, (), device: VkDevice, sampler: VkSampler, pAllocator: *const VkAllocationCallbacks);
    func!(DestroySemaphore, (), device: VkDevice, semaphore: VkSemaphore, pAllocator: *const VkAllocationCallbacks);
    func!(DestroyShaderModule, (), device: VkDevice, shaderModule: VkShaderModule, pAllocator: *const VkAllocationCallbacks);
    func!(DeviceWaitIdle, VkResult, device: VkDevice);
    func!(EndCommandBuffer, VkResult, commandBuffer: VkCommandBuffer);
    func!(EnumerateDeviceExtensionProperties, VkResult, physicalDevice: VkPhysicalDevice, pLayerName: *const c_char, pPropertyCount: *mut u32, pProperties: *mut VkExtensionProperties);
    func!(EnumerateDeviceLayerProperties, VkResult, physicalDevice: VkPhysicalDevice, pPropertyCount: *mut u32, pProperties: *mut VkLayerProperties);
    func!(EnumerateInstanceExtensionProperties, VkResult, pLayerName: *const c_char, pPropertyCount: *mut u32, pProperties: *mut VkExtensionProperties);
    func!(EnumerateInstanceLayerProperties, VkResult, pPropertyCount: *mut u32, pProperties: *mut VkLayerProperties);
    func!(EnumeratePhysicalDevices, VkResult, instance: VkInstance, pPhysicalDeviceCount: *mut u32, pPhysicalDevices: *mut VkPhysicalDevice);
    func!(FlushMappedMemoryRanges, VkResult, device: VkDevice, memoryRangeCount: u32, pMemoryRanges: *const VkMappedMemoryRange);
    func!(FreeCommandBuffers, (), device: VkDevice, commandPool: VkCommandPool, commandBufferCount: u32, pCommandBuffers: *const VkCommandBuffer);
    func!(FreeDescriptorSets, VkResult, device: VkDevice, descriptorPool: VkDescriptorPool, descriptorSetCount: u32, pDescriptorSets: *const VkDescriptorSet);
    func!(FreeMemory, (), device: VkDevice, memory: VkDeviceMemory, pAllocator: *const VkAllocationCallbacks);
    func!(GetBufferMemoryRequirements, (), device: VkDevice, buffer: VkBuffer, pMemoryRequirements: *mut VkMemoryRequirements);
    func!(GetDeviceMemoryCommitment, (), device: VkDevice, memory: VkDeviceMemory, pCommittedMemoryInBytes: *mut VkDeviceSize);
    func!(GetDeviceProcAddr, PFN_vkVoidFunction, device: VkDevice, pName: *const c_char);
    func!(GetDeviceQueue, (), device: VkDevice, queueFamilyIndex: u32, queueIndex: u32, pQueue: *mut VkQueue);
    func!(GetEventStatus, VkResult, device: VkDevice, event: VkEvent);
    func!(GetFenceStatus, VkResult, device: VkDevice, fence: VkFence);
    func!(GetImageMemoryRequirements, (), device: VkDevice, image: VkImage, pMemoryRequirements: *mut VkMemoryRequirements);
    func!(GetImageSparseMemoryRequirements, (), device: VkDevice, image: VkImage, pSparseMemoryRequirementCount: *mut u32, pSparseMemoryRequirements: *mut VkSparseImageMemoryRequirements);
    func!(GetImageSubresourceLayout, (), device: VkDevice, image: VkImage, pSubresource: *const VkImageSubresource, pLayout: *mut VkSubresourceLayout);
    func!(GetInstanceProcAddr, PFN_vkVoidFunction, instance: VkInstance, pName: *const c_char);
    #[cfg(any(feature = "VK_USE_PLATFORM_WIN32_KHR"))]
    func!(GetMemoryWin32HandleKHR, VkResult, device: VkDevice, pGetWin32HandleInfo: *const VkMemoryGetWin32HandleInfoKHR, pHandle: *mut HANDLE);
    #[cfg(any(feature = "VK_USE_PLATFORM_WIN32_KHR"))]
    func!(GetMemoryWin32HandlePropertiesKHR, VkResult, device: VkDevice, handleType: VkExternalMemoryHandleTypeFlagBits, handle: HANDLE, pMemoryWin32HandleProperties: *mut VkMemoryWin32HandlePropertiesKHR);
    func!(GetPhysicalDeviceExternalBufferPropertiesKHR, (), physicalDevice: VkPhysicalDevice, pExternalBufferInfo: *const VkPhysicalDeviceExternalBufferInfo, pExternalBufferProperties: *mut VkExternalBufferProperties);
    func!(GetPhysicalDeviceFeatures, (), physicalDevice: VkPhysicalDevice, pFeatures: *mut VkPhysicalDeviceFeatures);
    func!(GetPhysicalDeviceFormatProperties, (), physicalDevice: VkPhysicalDevice, format: VkFormat, pFormatProperties: *mut VkFormatProperties);
    func!(GetPhysicalDeviceImageFormatProperties, VkResult, physicalDevice: VkPhysicalDevice, format: VkFormat, type_: VkImageType, tiling: VkImageTiling, usage: VkImageUsageFlags, flags: VkImageCreateFlags, pImageFormatProperties: *mut VkImageFormatProperties);
    func!(GetPhysicalDeviceMemoryProperties, (), physicalDevice: VkPhysicalDevice, pMemoryProperties: *mut VkPhysicalDeviceMemoryProperties);
    func!(GetPhysicalDeviceProperties, (), physicalDevice: VkPhysicalDevice, pProperties: *mut VkPhysicalDeviceProperties);
    func!(GetPhysicalDeviceQueueFamilyProperties, (), physicalDevice: VkPhysicalDevice, pQueueFamilyPropertyCount: *mut u32, pQueueFamilyProperties: *mut VkQueueFamilyProperties);
    func!(GetPhysicalDeviceSparseImageFormatProperties, (), physicalDevice: VkPhysicalDevice, format: VkFormat, type_: VkImageType, samples: VkSampleCountFlagBits, usage: VkImageUsageFlags, tiling: VkImageTiling, pPropertyCount: *mut u32, pProperties: *mut VkSparseImageFormatProperties);
    func!(GetPipelineCacheData, VkResult, device: VkDevice, pipelineCache: VkPipelineCache, pDataSize: *mut usize, pData: *mut c_void);
    func!(GetQueryPoolResults, VkResult, device: VkDevice, queryPool: VkQueryPool, firstQuery: u32, queryCount: u32, dataSize: usize, pData: *mut c_void, stride: VkDeviceSize, flags: VkQueryResultFlags);
    func!(GetRenderAreaGranularity, (), device: VkDevice, renderPass: VkRenderPass, pGranularity: *mut VkExtent2D);
    func!(InvalidateMappedMemoryRanges, VkResult, device: VkDevice, memoryRangeCount: u32, pMemoryRanges: *const VkMappedMemoryRange);
    func!(MapMemory, VkResult, device: VkDevice, memory: VkDeviceMemory, offset: VkDeviceSize, size: VkDeviceSize, flags: VkMemoryMapFlags, ppData: *mut *mut c_void);
    func!(MergePipelineCaches, VkResult, device: VkDevice, dstCache: VkPipelineCache, srcCacheCount: u32, pSrcCaches: *const VkPipelineCache);
    func!(QueueBindSparse, VkResult, queue: VkQueue, bindInfoCount: u32, pBindInfo: *const VkBindSparseInfo, fence: VkFence);
    func!(QueueSubmit, VkResult, queue: VkQueue, submitCount: u32, pSubmits: *const VkSubmitInfo, fence: VkFence);
    func!(QueueWaitIdle, VkResult, queue: VkQueue);
    func!(ResetCommandBuffer, VkResult, commandBuffer: VkCommandBuffer, flags: VkCommandBufferResetFlags);
    func!(ResetCommandPool, VkResult, device: VkDevice, commandPool: VkCommandPool, flags: VkCommandPoolResetFlags);
    func!(ResetDescriptorPool, VkResult, device: VkDevice, descriptorPool: VkDescriptorPool, flags: VkDescriptorPoolResetFlags);
    func!(ResetEvent, VkResult, device: VkDevice, event: VkEvent);
    func!(ResetFences, VkResult, device: VkDevice, fenceCount: u32, pFences: *const VkFence);
    func!(SetEvent, VkResult, device: VkDevice, event: VkEvent);
    func!(UnmapMemory, (), device: VkDevice, memory: VkDeviceMemory);
    func!(UpdateDescriptorSets, (), device: VkDevice, descriptorWriteCount: u32, pDescriptorWrites: *const VkWriteDescriptorSet, descriptorCopyCount: u32, pDescriptorCopies: *const VkCopyDescriptorSet);
    func!(WaitForFences, VkResult, device: VkDevice, fenceCount: u32, pFences: *const VkFence, waitAll: VkBool32, timeout: u64);
}

mod storage {
    #![allow(non_snake_case, non_upper_case_globals)]

    use super::FnPtr;
    use std::cell::Cell;
    use std::os::raw::*;

    macro_rules! store {
        ($name:ident) => {
            pub(super) static mut $name: FnPtr = FnPtr {
                ptr: Cell::new(FnPtr::not_initialized as *const c_void),
                is_loaded: Cell::new(false),
            };
        };
    }

    store!(AllocateCommandBuffers);
    store!(AllocateDescriptorSets);
    store!(AllocateMemory);
    store!(BeginCommandBuffer);
    store!(BindBufferMemory);
    store!(BindImageMemory);
    store!(CmdBeginQuery);
    store!(CmdBeginRenderPass);
    store!(CmdBindDescriptorSets);
    store!(CmdBindIndexBuffer);
    store!(CmdBindPipeline);
    store!(CmdBindVertexBuffers);
    store!(CmdBlitImage);
    store!(CmdClearAttachments);
    store!(CmdClearColorImage);
    store!(CmdClearDepthStencilImage);
    store!(CmdCopyBuffer);
    store!(CmdCopyBufferToImage);
    store!(CmdCopyImage);
    store!(CmdCopyImageToBuffer);
    store!(CmdCopyQueryPoolResults);
    store!(CmdDispatch);
    store!(CmdDispatchIndirect);
    store!(CmdDraw);
    store!(CmdDrawIndexed);
    store!(CmdDrawIndexedIndirect);
    store!(CmdDrawIndirect);
    store!(CmdEndQuery);
    store!(CmdEndRenderPass);
    store!(CmdExecuteCommands);
    store!(CmdFillBuffer);
    store!(CmdNextSubpass);
    store!(CmdPipelineBarrier);
    store!(CmdPushConstants);
    store!(CmdResetEvent);
    store!(CmdResetQueryPool);
    store!(CmdResolveImage);
    store!(CmdSetBlendConstants);
    store!(CmdSetDepthBias);
    store!(CmdSetDepthBounds);
    store!(CmdSetEvent);
    store!(CmdSetLineWidth);
    store!(CmdSetScissor);
    store!(CmdSetStencilCompareMask);
    store!(CmdSetStencilReference);
    store!(CmdSetStencilWriteMask);
    store!(CmdSetViewport);
    store!(CmdUpdateBuffer);
    store!(CmdWaitEvents);
    store!(CmdWriteTimestamp);
    store!(CreateBuffer);
    store!(CreateBufferView);
    store!(CreateCommandPool);
    store!(CreateComputePipelines);
    store!(CreateDescriptorPool);
    store!(CreateDescriptorSetLayout);
    store!(CreateDevice);
    store!(CreateEvent);
    store!(CreateFence);
    store!(CreateFramebuffer);
    store!(CreateGraphicsPipelines);
    store!(CreateImage);
    store!(CreateImageView);
    store!(CreateInstance);
    store!(CreatePipelineCache);
    store!(CreatePipelineLayout);
    store!(CreateQueryPool);
    store!(CreateRenderPass);
    store!(CreateSampler);
    store!(CreateSemaphore);
    store!(CreateShaderModule);
    store!(DestroyBuffer);
    store!(DestroyBufferView);
    store!(DestroyCommandPool);
    store!(DestroyDescriptorPool);
    store!(DestroyDescriptorSetLayout);
    store!(DestroyDevice);
    store!(DestroyEvent);
    store!(DestroyFence);
    store!(DestroyFramebuffer);
    store!(DestroyImage);
    store!(DestroyImageView);
    store!(DestroyInstance);
    store!(DestroyPipeline);
    store!(DestroyPipelineCache);
    store!(DestroyPipelineLayout);
    store!(DestroyQueryPool);
    store!(DestroyRenderPass);
    store!(DestroySampler);
    store!(DestroySemaphore);
    store!(DestroyShaderModule);
    store!(DeviceWaitIdle);
    store!(EndCommandBuffer);
    store!(EnumerateDeviceExtensionProperties);
    store!(EnumerateDeviceLayerProperties);
    store!(EnumerateInstanceExtensionProperties);
    store!(EnumerateInstanceLayerProperties);
    store!(EnumeratePhysicalDevices);
    store!(FlushMappedMemoryRanges);
    store!(FreeCommandBuffers);
    store!(FreeDescriptorSets);
    store!(FreeMemory);
    store!(GetBufferMemoryRequirements);
    store!(GetDeviceMemoryCommitment);
    store!(GetDeviceProcAddr);
    store!(GetDeviceQueue);
    store!(GetEventStatus);
    store!(GetFenceStatus);
    store!(GetImageMemoryRequirements);
    store!(GetImageSparseMemoryRequirements);
    store!(GetImageSubresourceLayout);
    store!(GetInstanceProcAddr);
    #[cfg(any(feature = "VK_USE_PLATFORM_WIN32_KHR"))]
    store!(GetMemoryWin32HandleKHR);
    #[cfg(any(feature = "VK_USE_PLATFORM_WIN32_KHR"))]
    store!(GetMemoryWin32HandlePropertiesKHR);
    store!(GetPhysicalDeviceExternalBufferPropertiesKHR);
    store!(GetPhysicalDeviceFeatures);
    store!(GetPhysicalDeviceFormatProperties);
    store!(GetPhysicalDeviceImageFormatProperties);
    store!(GetPhysicalDeviceMemoryProperties);
    store!(GetPhysicalDeviceProperties);
    store!(GetPhysicalDeviceQueueFamilyProperties);
    store!(GetPhysicalDeviceSparseImageFormatProperties);
    store!(GetPipelineCacheData);
    store!(GetQueryPoolResults);
    store!(GetRenderAreaGranularity);
    store!(InvalidateMappedMemoryRanges);
    store!(MapMemory);
    store!(MergePipelineCaches);
    store!(QueueBindSparse);
    store!(QueueSubmit);
    store!(QueueWaitIdle);
    store!(ResetCommandBuffer);
    store!(ResetCommandPool);
    store!(ResetDescriptorPool);
    store!(ResetEvent);
    store!(ResetFences);
    store!(SetEvent);
    store!(UnmapMemory);
    store!(UpdateDescriptorSets);
    store!(WaitForFences);
}

pub fn load<F>(mut loadfn: F)
where
    F: FnMut(&'static str) -> *const c_void,
{
    unsafe {
        storage::AllocateCommandBuffers.set_ptr(loadfn("vkAllocateCommandBuffers"));
        storage::AllocateDescriptorSets.set_ptr(loadfn("vkAllocateDescriptorSets"));
        storage::AllocateMemory.set_ptr(loadfn("vkAllocateMemory"));
        storage::BeginCommandBuffer.set_ptr(loadfn("vkBeginCommandBuffer"));
        storage::BindBufferMemory.set_ptr(loadfn("vkBindBufferMemory"));
        storage::BindImageMemory.set_ptr(loadfn("vkBindImageMemory"));
        storage::CmdBeginQuery.set_ptr(loadfn("vkCmdBeginQuery"));
        storage::CmdBeginRenderPass.set_ptr(loadfn("vkCmdBeginRenderPass"));
        storage::CmdBindDescriptorSets.set_ptr(loadfn("vkCmdBindDescriptorSets"));
        storage::CmdBindIndexBuffer.set_ptr(loadfn("vkCmdBindIndexBuffer"));
        storage::CmdBindPipeline.set_ptr(loadfn("vkCmdBindPipeline"));
        storage::CmdBindVertexBuffers.set_ptr(loadfn("vkCmdBindVertexBuffers"));
        storage::CmdBlitImage.set_ptr(loadfn("vkCmdBlitImage"));
        storage::CmdClearAttachments.set_ptr(loadfn("vkCmdClearAttachments"));
        storage::CmdClearColorImage.set_ptr(loadfn("vkCmdClearColorImage"));
        storage::CmdClearDepthStencilImage.set_ptr(loadfn("vkCmdClearDepthStencilImage"));
        storage::CmdCopyBuffer.set_ptr(loadfn("vkCmdCopyBuffer"));
        storage::CmdCopyBufferToImage.set_ptr(loadfn("vkCmdCopyBufferToImage"));
        storage::CmdCopyImage.set_ptr(loadfn("vkCmdCopyImage"));
        storage::CmdCopyImageToBuffer.set_ptr(loadfn("vkCmdCopyImageToBuffer"));
        storage::CmdCopyQueryPoolResults.set_ptr(loadfn("vkCmdCopyQueryPoolResults"));
        storage::CmdDispatch.set_ptr(loadfn("vkCmdDispatch"));
        storage::CmdDispatchIndirect.set_ptr(loadfn("vkCmdDispatchIndirect"));
        storage::CmdDraw.set_ptr(loadfn("vkCmdDraw"));
        storage::CmdDrawIndexed.set_ptr(loadfn("vkCmdDrawIndexed"));
        storage::CmdDrawIndexedIndirect.set_ptr(loadfn("vkCmdDrawIndexedIndirect"));
        storage::CmdDrawIndirect.set_ptr(loadfn("vkCmdDrawIndirect"));
        storage::CmdEndQuery.set_ptr(loadfn("vkCmdEndQuery"));
        storage::CmdEndRenderPass.set_ptr(loadfn("vkCmdEndRenderPass"));
        storage::CmdExecuteCommands.set_ptr(loadfn("vkCmdExecuteCommands"));
        storage::CmdFillBuffer.set_ptr(loadfn("vkCmdFillBuffer"));
        storage::CmdNextSubpass.set_ptr(loadfn("vkCmdNextSubpass"));
        storage::CmdPipelineBarrier.set_ptr(loadfn("vkCmdPipelineBarrier"));
        storage::CmdPushConstants.set_ptr(loadfn("vkCmdPushConstants"));
        storage::CmdResetEvent.set_ptr(loadfn("vkCmdResetEvent"));
        storage::CmdResetQueryPool.set_ptr(loadfn("vkCmdResetQueryPool"));
        storage::CmdResolveImage.set_ptr(loadfn("vkCmdResolveImage"));
        storage::CmdSetBlendConstants.set_ptr(loadfn("vkCmdSetBlendConstants"));
        storage::CmdSetDepthBias.set_ptr(loadfn("vkCmdSetDepthBias"));
        storage::CmdSetDepthBounds.set_ptr(loadfn("vkCmdSetDepthBounds"));
        storage::CmdSetEvent.set_ptr(loadfn("vkCmdSetEvent"));
        storage::CmdSetLineWidth.set_ptr(loadfn("vkCmdSetLineWidth"));
        storage::CmdSetScissor.set_ptr(loadfn("vkCmdSetScissor"));
        storage::CmdSetStencilCompareMask.set_ptr(loadfn("vkCmdSetStencilCompareMask"));
        storage::CmdSetStencilReference.set_ptr(loadfn("vkCmdSetStencilReference"));
        storage::CmdSetStencilWriteMask.set_ptr(loadfn("vkCmdSetStencilWriteMask"));
        storage::CmdSetViewport.set_ptr(loadfn("vkCmdSetViewport"));
        storage::CmdUpdateBuffer.set_ptr(loadfn("vkCmdUpdateBuffer"));
        storage::CmdWaitEvents.set_ptr(loadfn("vkCmdWaitEvents"));
        storage::CmdWriteTimestamp.set_ptr(loadfn("vkCmdWriteTimestamp"));
        storage::CreateBuffer.set_ptr(loadfn("vkCreateBuffer"));
        storage::CreateBufferView.set_ptr(loadfn("vkCreateBufferView"));
        storage::CreateCommandPool.set_ptr(loadfn("vkCreateCommandPool"));
        storage::CreateComputePipelines.set_ptr(loadfn("vkCreateComputePipelines"));
        storage::CreateDescriptorPool.set_ptr(loadfn("vkCreateDescriptorPool"));
        storage::CreateDescriptorSetLayout.set_ptr(loadfn("vkCreateDescriptorSetLayout"));
        storage::CreateDevice.set_ptr(loadfn("vkCreateDevice"));
        storage::CreateEvent.set_ptr(loadfn("vkCreateEvent"));
        storage::CreateFence.set_ptr(loadfn("vkCreateFence"));
        storage::CreateFramebuffer.set_ptr(loadfn("vkCreateFramebuffer"));
        storage::CreateGraphicsPipelines.set_ptr(loadfn("vkCreateGraphicsPipelines"));
        storage::CreateImage.set_ptr(loadfn("vkCreateImage"));
        storage::CreateImageView.set_ptr(loadfn("vkCreateImageView"));
        storage::CreateInstance.set_ptr(loadfn("vkCreateInstance"));
        storage::CreatePipelineCache.set_ptr(loadfn("vkCreatePipelineCache"));
        storage::CreatePipelineLayout.set_ptr(loadfn("vkCreatePipelineLayout"));
        storage::CreateQueryPool.set_ptr(loadfn("vkCreateQueryPool"));
        storage::CreateRenderPass.set_ptr(loadfn("vkCreateRenderPass"));
        storage::CreateSampler.set_ptr(loadfn("vkCreateSampler"));
        storage::CreateSemaphore.set_ptr(loadfn("vkCreateSemaphore"));
        storage::CreateShaderModule.set_ptr(loadfn("vkCreateShaderModule"));
        storage::DestroyBuffer.set_ptr(loadfn("vkDestroyBuffer"));
        storage::DestroyBufferView.set_ptr(loadfn("vkDestroyBufferView"));
        storage::DestroyCommandPool.set_ptr(loadfn("vkDestroyCommandPool"));
        storage::DestroyDescriptorPool.set_ptr(loadfn("vkDestroyDescriptorPool"));
        storage::DestroyDescriptorSetLayout.set_ptr(loadfn("vkDestroyDescriptorSetLayout"));
        storage::DestroyDevice.set_ptr(loadfn("vkDestroyDevice"));
        storage::DestroyEvent.set_ptr(loadfn("vkDestroyEvent"));
        storage::DestroyFence.set_ptr(loadfn("vkDestroyFence"));
        storage::DestroyFramebuffer.set_ptr(loadfn("vkDestroyFramebuffer"));
        storage::DestroyImage.set_ptr(loadfn("vkDestroyImage"));
        storage::DestroyImageView.set_ptr(loadfn("vkDestroyImageView"));
        storage::DestroyInstance.set_ptr(loadfn("vkDestroyInstance"));
        storage::DestroyPipeline.set_ptr(loadfn("vkDestroyPipeline"));
        storage::DestroyPipelineCache.set_ptr(loadfn("vkDestroyPipelineCache"));
        storage::DestroyPipelineLayout.set_ptr(loadfn("vkDestroyPipelineLayout"));
        storage::DestroyQueryPool.set_ptr(loadfn("vkDestroyQueryPool"));
        storage::DestroyRenderPass.set_ptr(loadfn("vkDestroyRenderPass"));
        storage::DestroySampler.set_ptr(loadfn("vkDestroySampler"));
        storage::DestroySemaphore.set_ptr(loadfn("vkDestroySemaphore"));
        storage::DestroyShaderModule.set_ptr(loadfn("vkDestroyShaderModule"));
        storage::DeviceWaitIdle.set_ptr(loadfn("vkDeviceWaitIdle"));
        storage::EndCommandBuffer.set_ptr(loadfn("vkEndCommandBuffer"));
        storage::EnumerateDeviceExtensionProperties
            .set_ptr(loadfn("vkEnumerateDeviceExtensionProperties"));
        storage::EnumerateDeviceLayerProperties.set_ptr(loadfn("vkEnumerateDeviceLayerProperties"));
        storage::EnumerateInstanceExtensionProperties
            .set_ptr(loadfn("vkEnumerateInstanceExtensionProperties"));
        storage::EnumerateInstanceLayerProperties
            .set_ptr(loadfn("vkEnumerateInstanceLayerProperties"));
        storage::EnumeratePhysicalDevices.set_ptr(loadfn("vkEnumeratePhysicalDevices"));
        storage::FlushMappedMemoryRanges.set_ptr(loadfn("vkFlushMappedMemoryRanges"));
        storage::FreeCommandBuffers.set_ptr(loadfn("vkFreeCommandBuffers"));
        storage::FreeDescriptorSets.set_ptr(loadfn("vkFreeDescriptorSets"));
        storage::FreeMemory.set_ptr(loadfn("vkFreeMemory"));
        storage::GetBufferMemoryRequirements.set_ptr(loadfn("vkGetBufferMemoryRequirements"));
        storage::GetDeviceMemoryCommitment.set_ptr(loadfn("vkGetDeviceMemoryCommitment"));
        storage::GetDeviceProcAddr.set_ptr(loadfn("vkGetDeviceProcAddr"));
        storage::GetDeviceQueue.set_ptr(loadfn("vkGetDeviceQueue"));
        storage::GetEventStatus.set_ptr(loadfn("vkGetEventStatus"));
        storage::GetFenceStatus.set_ptr(loadfn("vkGetFenceStatus"));
        storage::GetImageMemoryRequirements.set_ptr(loadfn("vkGetImageMemoryRequirements"));
        storage::GetImageSparseMemoryRequirements
            .set_ptr(loadfn("vkGetImageSparseMemoryRequirements"));
        storage::GetImageSubresourceLayout.set_ptr(loadfn("vkGetImageSubresourceLayout"));
        storage::GetInstanceProcAddr.set_ptr(loadfn("vkGetInstanceProcAddr"));
        #[cfg(any(feature = "VK_USE_PLATFORM_WIN32_KHR"))]
        storage::GetMemoryWin32HandleKHR.set_ptr(loadfn("vkGetMemoryWin32HandleKHR"));
        #[cfg(any(feature = "VK_USE_PLATFORM_WIN32_KHR"))]
        storage::GetMemoryWin32HandlePropertiesKHR
            .set_ptr(loadfn("vkGetMemoryWin32HandlePropertiesKHR"));
        storage::GetPhysicalDeviceExternalBufferPropertiesKHR
            .set_ptr(loadfn("vkGetPhysicalDeviceExternalBufferPropertiesKHR"));
        storage::GetPhysicalDeviceFeatures.set_ptr(loadfn("vkGetPhysicalDeviceFeatures"));
        storage::GetPhysicalDeviceFormatProperties
            .set_ptr(loadfn("vkGetPhysicalDeviceFormatProperties"));
        storage::GetPhysicalDeviceImageFormatProperties
            .set_ptr(loadfn("vkGetPhysicalDeviceImageFormatProperties"));
        storage::GetPhysicalDeviceMemoryProperties
            .set_ptr(loadfn("vkGetPhysicalDeviceMemoryProperties"));
        storage::GetPhysicalDeviceProperties.set_ptr(loadfn("vkGetPhysicalDeviceProperties"));
        storage::GetPhysicalDeviceQueueFamilyProperties
            .set_ptr(loadfn("vkGetPhysicalDeviceQueueFamilyProperties"));
        storage::GetPhysicalDeviceSparseImageFormatProperties
            .set_ptr(loadfn("vkGetPhysicalDeviceSparseImageFormatProperties"));
        storage::GetPipelineCacheData.set_ptr(loadfn("vkGetPipelineCacheData"));
        storage::GetQueryPoolResults.set_ptr(loadfn("vkGetQueryPoolResults"));
        storage::GetRenderAreaGranularity.set_ptr(loadfn("vkGetRenderAreaGranularity"));
        storage::InvalidateMappedMemoryRanges.set_ptr(loadfn("vkInvalidateMappedMemoryRanges"));
        storage::MapMemory.set_ptr(loadfn("vkMapMemory"));
        storage::MergePipelineCaches.set_ptr(loadfn("vkMergePipelineCaches"));
        storage::QueueBindSparse.set_ptr(loadfn("vkQueueBindSparse"));
        storage::QueueSubmit.set_ptr(loadfn("vkQueueSubmit"));
        storage::QueueWaitIdle.set_ptr(loadfn("vkQueueWaitIdle"));
        storage::ResetCommandBuffer.set_ptr(loadfn("vkResetCommandBuffer"));
        storage::ResetCommandPool.set_ptr(loadfn("vkResetCommandPool"));
        storage::ResetDescriptorPool.set_ptr(loadfn("vkResetDescriptorPool"));
        storage::ResetEvent.set_ptr(loadfn("vkResetEvent"));
        storage::ResetFences.set_ptr(loadfn("vkResetFences"));
        storage::SetEvent.set_ptr(loadfn("vkSetEvent"));
        storage::UnmapMemory.set_ptr(loadfn("vkUnmapMemory"));
        storage::UpdateDescriptorSets.set_ptr(loadfn("vkUpdateDescriptorSets"));
        storage::WaitForFences.set_ptr(loadfn("vkWaitForFences"));
    }
}
