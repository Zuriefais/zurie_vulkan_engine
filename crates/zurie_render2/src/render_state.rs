use crate::constants::*;
use crate::debug;
use crate::debug::setup_debug_utils;
use crate::platforms;
use crate::structures::*;
use crate::structures::*;
use crate::tools;
use ash::vk;
use egui::{ClippedPrimitive, Context, TextureId, ViewportId};
use egui_ash_renderer::{Options, Renderer};
use egui_winit::State;
use log::info;
use naga::back::spv; // For generating SPIR-V
use naga::front::wgsl; // For parsing WGSL
use naga::valid::{Capabilities, ValidationFlags, Validator};
use std::ffi::CString;
use std::ffi::c_char;
use std::ffi::c_void;
use std::ptr;
use std::sync::Arc;
use winit::application::ApplicationHandler;
use winit::event::KeyEvent;
use winit::event::WindowEvent::KeyboardInput;
use winit::event::{ElementState, Event, WindowEvent};
use winit::event_loop;
use winit::event_loop::ActiveEventLoop;
use winit::event_loop::{ControlFlow, EventLoop};
use winit::keyboard::{KeyCode, PhysicalKey};
use winit::raw_window_handle::HasDisplayHandle;
use winit::window::Window;
use winit::window::WindowId;
// Constants
const WINDOW_TITLE: &'static str = "15.Hello Triangle";
const MAX_FRAMES_IN_FLIGHT: usize = 2;

struct SyncObjects {
    image_available_semaphores: Vec<vk::Semaphore>,
    render_finished_semaphores: Vec<vk::Semaphore>,
    inflight_fences: Vec<vk::Fence>,
}

struct RenderState {
    window: Arc<Window>,
    // vulkan stuff
    _entry: ash::Entry,
    instance: ash::Instance,
    surface_loader: ash::khr::surface::Instance,
    surface: vk::SurfaceKHR,
    debug_utils_loader: ash::ext::debug_utils::Instance,
    debug_merssager: vk::DebugUtilsMessengerEXT,

    _physical_device: vk::PhysicalDevice,
    device: ash::Device,

    graphics_queue: vk::Queue,
    present_queue: vk::Queue,

    swapchain_loader: ash::khr::swapchain::Device,
    swapchain: vk::SwapchainKHR,
    _swapchain_images: Vec<vk::Image>,
    _swapchain_format: vk::Format,
    _swapchain_extent: vk::Extent2D,
    swapchain_imageviews: Vec<vk::ImageView>,
    swapchain_framebuffers: Vec<vk::Framebuffer>,

    render_pass: vk::RenderPass,
    pipeline_layout: vk::PipelineLayout,
    graphics_pipeline: vk::Pipeline,

    command_pool: vk::CommandPool,
    command_buffers: Vec<vk::CommandBuffer>,

    image_available_semaphores: Vec<vk::Semaphore>,
    render_finished_semaphores: Vec<vk::Semaphore>,
    in_flight_fences: Vec<vk::Fence>,
    current_frame: usize,

    egui_ctx: Context,
    egui_winit: State,
    egui_renderer: Renderer,
    textures_to_free: Option<Vec<TextureId>>,
}

pub fn create_surface(
    entry: &ash::Entry,
    instance: &ash::Instance,
    window: Arc<Window>,
    screen_width: u32,
    screen_height: u32,
) -> SurfaceStuff {
    let surface = unsafe {
        platforms::create_surface(entry, instance, window).expect("Failed to create surface.")
    };
    let surface_loader = ash::khr::surface::Instance::new(entry, instance);

    SurfaceStuff {
        surface_loader,
        surface,
        screen_width,
        screen_height,
    }
}

pub fn create_instance(
    entry: &ash::Entry,
    window_title: &str,
    is_enable_debug: bool,
    required_validation_layers: &Vec<&str>,
) -> ash::Instance {
    if is_enable_debug
        && debug::check_validation_layer_support(entry, required_validation_layers) == false
    {
        panic!("Validation layers requested, but not available!");
    }

    let app_name = CString::new(window_title).unwrap();
    let engine_name = CString::new("Vulkan Engine").unwrap();
    let app_info = vk::ApplicationInfo {
        p_application_name: app_name.as_ptr(),
        s_type: vk::StructureType::APPLICATION_INFO,
        p_next: ptr::null(),
        application_version: APPLICATION_VERSION,
        p_engine_name: engine_name.as_ptr(),
        engine_version: ENGINE_VERSION,
        api_version: API_VERSION,
        _marker: std::marker::PhantomData,
    };

    // This create info used to debug issues in vk::createInstance and vk::destroyInstance.
    let debug_utils_create_info = debug::populate_debug_messenger_create_info();

    // VK_EXT debug report has been requested here.
    let extension_names = platforms::required_extension_names();

    let requred_validation_layer_raw_names: Vec<CString> = required_validation_layers
        .iter()
        .map(|layer_name| CString::new(*layer_name).unwrap())
        .collect();
    let layer_names: Vec<*const i8> = requred_validation_layer_raw_names
        .iter()
        .map(|layer_name| layer_name.as_ptr())
        .collect();

    let create_info = vk::InstanceCreateInfo {
        s_type: vk::StructureType::INSTANCE_CREATE_INFO,
        p_next: if VALIDATION.is_enable {
            &debug_utils_create_info as *const vk::DebugUtilsMessengerCreateInfoEXT as *const c_void
        } else {
            ptr::null()
        },
        flags: vk::InstanceCreateFlags::empty(),
        p_application_info: &app_info,
        pp_enabled_layer_names: if is_enable_debug {
            layer_names.as_ptr()
        } else {
            ptr::null()
        },
        enabled_layer_count: if is_enable_debug {
            layer_names.len()
        } else {
            0
        } as u32,
        pp_enabled_extension_names: extension_names.as_ptr(),
        enabled_extension_count: extension_names.len() as u32,
        _marker: std::marker::PhantomData,
    };

    let instance: ash::Instance = unsafe {
        entry
            .create_instance(&create_info, None)
            .expect("Failed to create instance!")
    };

    instance
}

pub fn pick_physical_device(
    instance: &ash::Instance,
    surface_stuff: &SurfaceStuff,
    required_device_extensions: &DeviceExtension,
) -> vk::PhysicalDevice {
    let physical_devices = unsafe {
        instance
            .enumerate_physical_devices()
            .expect("Failed to enumerate Physical Devices!")
    };

    let result = physical_devices.iter().find(|physical_device| {
        let is_suitable = is_physical_device_suitable(
            instance,
            **physical_device,
            surface_stuff,
            required_device_extensions,
        );

        // if is_suitable {
        //     let device_properties = instance.get_physical_device_properties(**physical_device);
        //     let device_name = super::tools::vk_to_string(&device_properties.device_name);
        //     println!("Using GPU: {}", device_name);
        // }

        is_suitable
    });

    match result {
        Some(p_physical_device) => *p_physical_device,
        None => panic!("Failed to find a suitable GPU!"),
    }
}

pub fn is_physical_device_suitable(
    instance: &ash::Instance,
    physical_device: vk::PhysicalDevice,
    surface_stuff: &SurfaceStuff,
    required_device_extensions: &DeviceExtension,
) -> bool {
    let device_features = unsafe { instance.get_physical_device_features(physical_device) };

    let indices = find_queue_family(instance, physical_device, surface_stuff);

    let is_queue_family_supported = indices.is_complete();
    let is_device_extension_supported =
        check_device_extension_support(instance, physical_device, required_device_extensions);
    let is_swapchain_supported = if is_device_extension_supported {
        let swapchain_support = query_swapchain_support(physical_device, surface_stuff);
        !swapchain_support.formats.is_empty() && !swapchain_support.present_modes.is_empty()
    } else {
        false
    };
    let is_support_sampler_anisotropy = device_features.sampler_anisotropy == 1;

    return is_queue_family_supported
        && is_device_extension_supported
        && is_swapchain_supported
        && is_support_sampler_anisotropy;
}

pub fn create_logical_device(
    instance: &ash::Instance,
    physical_device: vk::PhysicalDevice,
    validation: &debug::ValidationInfo,
    device_extensions: &DeviceExtension,
    surface_stuff: &SurfaceStuff,
) -> (ash::Device, QueueFamilyIndices) {
    let indices = find_queue_family(instance, physical_device, surface_stuff);

    use std::collections::HashSet;
    let mut unique_queue_families = HashSet::new();
    unique_queue_families.insert(indices.graphics_family.unwrap());
    unique_queue_families.insert(indices.present_family.unwrap());

    let queue_priorities = [1.0_f32];
    let mut queue_create_infos = vec![];
    for &queue_family in unique_queue_families.iter() {
        let queue_create_info = vk::DeviceQueueCreateInfo {
            s_type: vk::StructureType::DEVICE_QUEUE_CREATE_INFO,
            p_next: ptr::null(),
            flags: vk::DeviceQueueCreateFlags::empty(),
            queue_family_index: queue_family,
            p_queue_priorities: queue_priorities.as_ptr(),
            queue_count: queue_priorities.len() as u32,
            _marker: std::marker::PhantomData,
        };
        queue_create_infos.push(queue_create_info);
    }

    let physical_device_features = vk::PhysicalDeviceFeatures {
        sampler_anisotropy: vk::TRUE, // enable anisotropy device feature from Chapter-24.
        ..Default::default()
    };

    let requred_validation_layer_raw_names: Vec<CString> = validation
        .required_validation_layers
        .iter()
        .map(|layer_name| CString::new(*layer_name).unwrap())
        .collect();
    let enable_layer_names: Vec<*const c_char> = requred_validation_layer_raw_names
        .iter()
        .map(|layer_name| layer_name.as_ptr())
        .collect();

    let enable_extension_names = CString::new(device_extensions.names[0]).unwrap();

    let device_create_info = vk::DeviceCreateInfo {
        s_type: vk::StructureType::DEVICE_CREATE_INFO,
        p_next: ptr::null(),
        flags: vk::DeviceCreateFlags::empty(),
        queue_create_info_count: queue_create_infos.len() as u32,
        p_queue_create_infos: queue_create_infos.as_ptr(),
        enabled_layer_count: if validation.is_enable {
            enable_layer_names.len()
        } else {
            0
        } as u32,
        pp_enabled_layer_names: if validation.is_enable {
            enable_layer_names.as_ptr()
        } else {
            ptr::null()
        },
        enabled_extension_count: device_extensions.names.len() as u32,
        pp_enabled_extension_names: &enable_extension_names.as_ptr(),
        p_enabled_features: &physical_device_features,
        _marker: std::marker::PhantomData,
    };

    let device: ash::Device = unsafe {
        instance
            .create_device(physical_device, &device_create_info, None)
            .expect("Failed to create logical Device!")
    };

    (device, indices)
}

pub fn find_queue_family(
    instance: &ash::Instance,
    physical_device: vk::PhysicalDevice,
    surface_stuff: &SurfaceStuff,
) -> QueueFamilyIndices {
    let queue_families =
        unsafe { instance.get_physical_device_queue_family_properties(physical_device) };

    let mut queue_family_indices = QueueFamilyIndices::new();

    let mut index = 0;
    for queue_family in queue_families.iter() {
        if queue_family.queue_count > 0
            && queue_family.queue_flags.contains(vk::QueueFlags::GRAPHICS)
        {
            queue_family_indices.graphics_family = Some(index);
        }

        let is_present_support = unsafe {
            surface_stuff
                .surface_loader
                .get_physical_device_surface_support(
                    physical_device,
                    index as u32,
                    surface_stuff.surface,
                )
                .unwrap()
        };
        if queue_family.queue_count > 0 && is_present_support {
            queue_family_indices.present_family = Some(index);
        }

        if queue_family_indices.is_complete() {
            break;
        }

        index += 1;
    }

    queue_family_indices
}

pub fn check_device_extension_support(
    instance: &ash::Instance,
    physical_device: vk::PhysicalDevice,
    device_extensions: &DeviceExtension,
) -> bool {
    let available_extensions = unsafe {
        instance
            .enumerate_device_extension_properties(physical_device)
            .expect("Failed to get device extension properties.")
    };

    let mut available_extension_names = vec![];

    for extension in available_extensions.iter() {
        let extension_name = tools::vk_to_string(&extension.extension_name);

        available_extension_names.push(extension_name);
    }

    use std::collections::HashSet;
    let mut required_extensions = HashSet::new();
    for extension in device_extensions.names.iter() {
        required_extensions.insert(extension.to_string());
    }

    for extension_name in available_extension_names.iter() {
        required_extensions.remove(extension_name);
    }

    return required_extensions.is_empty();
}

pub fn query_swapchain_support(
    physical_device: vk::PhysicalDevice,
    surface_stuff: &SurfaceStuff,
) -> SwapChainSupportDetail {
    unsafe {
        let capabilities = surface_stuff
            .surface_loader
            .get_physical_device_surface_capabilities(physical_device, surface_stuff.surface)
            .expect("Failed to query for surface capabilities.");
        let formats = surface_stuff
            .surface_loader
            .get_physical_device_surface_formats(physical_device, surface_stuff.surface)
            .expect("Failed to query for surface formats.");
        let present_modes = surface_stuff
            .surface_loader
            .get_physical_device_surface_present_modes(physical_device, surface_stuff.surface)
            .expect("Failed to query for surface present mode.");

        SwapChainSupportDetail {
            capabilities,
            formats,
            present_modes,
        }
    }
}

pub fn create_swapchain(
    instance: &ash::Instance,
    device: &ash::Device,
    physical_device: vk::PhysicalDevice,
    window: &Arc<Window>,
    surface_stuff: &SurfaceStuff,
    queue_family: &QueueFamilyIndices,
) -> SwapChainStuff {
    let swapchain_support = query_swapchain_support(physical_device, surface_stuff);
    let surface_format = choose_swapchain_format(&swapchain_support.formats);
    info!(
        "Supported present modes: {:?}",
        swapchain_support.present_modes
    );
    let present_mode = choose_swapchain_present_mode(&swapchain_support.present_modes);
    info!("Selected present mode: {:?}", present_mode);
    let extent = choose_swapchain_extent(&swapchain_support.capabilities, window);
    let image_count = swapchain_support.capabilities.min_image_count + 1;
    let image_count = if swapchain_support.capabilities.max_image_count > 0 {
        image_count.min(swapchain_support.capabilities.max_image_count)
    } else {
        image_count
    };
    let (image_sharing_mode, queue_family_index_count, queue_family_indices) =
        if queue_family.graphics_family != queue_family.present_family {
            (vk::SharingMode::CONCURRENT, 2, vec![
                queue_family.graphics_family.unwrap(),
                queue_family.present_family.unwrap(),
            ])
        } else {
            (vk::SharingMode::EXCLUSIVE, 0, vec![])
        };
    let swapchain_create_info = vk::SwapchainCreateInfoKHR {
        s_type: vk::StructureType::SWAPCHAIN_CREATE_INFO_KHR,
        p_next: ptr::null(),
        flags: vk::SwapchainCreateFlagsKHR::empty(),
        surface: surface_stuff.surface,
        min_image_count: image_count,
        image_color_space: surface_format.color_space,
        image_format: surface_format.format,
        image_extent: extent,
        image_usage: vk::ImageUsageFlags::COLOR_ATTACHMENT,
        image_sharing_mode,
        p_queue_family_indices: queue_family_indices.as_ptr(),
        queue_family_index_count,
        pre_transform: swapchain_support.capabilities.current_transform,
        composite_alpha: vk::CompositeAlphaFlagsKHR::OPAQUE,
        present_mode,
        clipped: vk::TRUE,
        old_swapchain: vk::SwapchainKHR::null(),
        image_array_layers: 1,
        _marker: std::marker::PhantomData,
    };
    let swapchain_loader = ash::khr::swapchain::Device::new(instance, device);
    let swapchain = unsafe {
        swapchain_loader
            .create_swapchain(&swapchain_create_info, None)
            .expect("Failed to create Swapchain!")
    };
    let swapchain_images = unsafe {
        swapchain_loader
            .get_swapchain_images(swapchain)
            .expect("Failed to get Swapchain Images.")
    };
    SwapChainStuff {
        swapchain_loader,
        swapchain,
        swapchain_format: surface_format.format,
        swapchain_extent: extent,
        swapchain_images,
    }
}

pub fn choose_swapchain_format(
    available_formats: &Vec<vk::SurfaceFormatKHR>,
) -> vk::SurfaceFormatKHR {
    for available_format in available_formats {
        if available_format.format == vk::Format::B8G8R8A8_SRGB
            && available_format.color_space == vk::ColorSpaceKHR::SRGB_NONLINEAR
        {
            return available_format.clone();
        }
    }

    return available_formats.first().unwrap().clone();
}

pub fn choose_swapchain_present_mode(
    available_present_modes: &Vec<vk::PresentModeKHR>,
) -> vk::PresentModeKHR {
    for &available_present_mode in available_present_modes.iter() {
        if available_present_mode == vk::PresentModeKHR::FIFO {
            return available_present_mode;
        }
    }

    vk::PresentModeKHR::FIFO
}

pub fn choose_swapchain_extent(
    capabilities: &vk::SurfaceCapabilitiesKHR,
    window: &Arc<Window>,
) -> vk::Extent2D {
    if capabilities.current_extent.width != u32::max_value() {
        capabilities.current_extent
    } else {
        use num::clamp;

        let window_size = window.inner_size();
        info!(
            "Inner Window Size: ({}, {})",
            window_size.width, window_size.height
        );

        vk::Extent2D {
            width: clamp(
                window_size.width as u32,
                capabilities.min_image_extent.width,
                capabilities.max_image_extent.width,
            ),
            height: clamp(
                window_size.height as u32,
                capabilities.min_image_extent.height,
                capabilities.max_image_extent.height,
            ),
        }
    }
}

pub fn create_image_views(
    device: &ash::Device,
    surface_format: vk::Format,
    images: &Vec<vk::Image>,
) -> Vec<vk::ImageView> {
    let swapchain_imageviews: Vec<vk::ImageView> = images
        .iter()
        .map(|&image| {
            create_image_view(
                device,
                image,
                surface_format,
                vk::ImageAspectFlags::COLOR,
                1,
            )
        })
        .collect();

    swapchain_imageviews
}

pub fn create_image_view(
    device: &ash::Device,
    image: vk::Image,
    format: vk::Format,
    aspect_flags: vk::ImageAspectFlags,
    mip_levels: u32,
) -> vk::ImageView {
    let imageview_create_info = vk::ImageViewCreateInfo {
        s_type: vk::StructureType::IMAGE_VIEW_CREATE_INFO,
        p_next: ptr::null(),
        flags: vk::ImageViewCreateFlags::empty(),
        view_type: vk::ImageViewType::TYPE_2D,
        format,
        components: vk::ComponentMapping {
            r: vk::ComponentSwizzle::IDENTITY,
            g: vk::ComponentSwizzle::IDENTITY,
            b: vk::ComponentSwizzle::IDENTITY,
            a: vk::ComponentSwizzle::IDENTITY,
        },
        subresource_range: vk::ImageSubresourceRange {
            aspect_mask: aspect_flags,
            base_mip_level: 0,
            level_count: mip_levels,
            base_array_layer: 0,
            layer_count: 1,
        },
        image,
        _marker: std::marker::PhantomData,
    };

    unsafe {
        device
            .create_image_view(&imageview_create_info, None)
            .expect("Failed to create Image View!")
    }
}

pub fn create_graphics_pipeline(
    device: &ash::Device,
    render_pass: vk::RenderPass,
    swapchain_extent: vk::Extent2D,
) -> (vk::Pipeline, vk::PipelineLayout) {
    let vert_shader_module = create_shader_module(device, include_str!("shaders/vert.wgsl"));
    let frag_shader_module = create_shader_module(device, include_str!("shaders/frag.wgsl"));

    let main_function_name = CString::new("main").unwrap();

    let shader_stages = [
        vk::PipelineShaderStageCreateInfo {
            s_type: vk::StructureType::PIPELINE_SHADER_STAGE_CREATE_INFO,
            p_next: ptr::null(),
            flags: vk::PipelineShaderStageCreateFlags::empty(),
            module: vert_shader_module,
            p_name: main_function_name.as_ptr(),
            p_specialization_info: ptr::null(),
            stage: vk::ShaderStageFlags::VERTEX,
            _marker: std::marker::PhantomData,
        },
        vk::PipelineShaderStageCreateInfo {
            s_type: vk::StructureType::PIPELINE_SHADER_STAGE_CREATE_INFO,
            p_next: ptr::null(),
            flags: vk::PipelineShaderStageCreateFlags::empty(),
            module: frag_shader_module,
            p_name: main_function_name.as_ptr(),
            p_specialization_info: ptr::null(),
            stage: vk::ShaderStageFlags::FRAGMENT,
            _marker: std::marker::PhantomData,
        },
    ];

    let vertex_input_state_create_info = vk::PipelineVertexInputStateCreateInfo {
        s_type: vk::StructureType::PIPELINE_VERTEX_INPUT_STATE_CREATE_INFO,
        p_next: ptr::null(),
        flags: vk::PipelineVertexInputStateCreateFlags::empty(),
        vertex_attribute_description_count: 0,
        p_vertex_attribute_descriptions: ptr::null(),
        vertex_binding_description_count: 0,
        p_vertex_binding_descriptions: ptr::null(),
        _marker: std::marker::PhantomData,
    };
    let vertex_input_assembly_state_info = vk::PipelineInputAssemblyStateCreateInfo {
        s_type: vk::StructureType::PIPELINE_INPUT_ASSEMBLY_STATE_CREATE_INFO,
        flags: vk::PipelineInputAssemblyStateCreateFlags::empty(),
        p_next: ptr::null(),
        primitive_restart_enable: vk::FALSE,
        topology: vk::PrimitiveTopology::TRIANGLE_LIST,
        _marker: std::marker::PhantomData,
    };

    // Define dynamic states
    let dynamic_states = [vk::DynamicState::VIEWPORT, vk::DynamicState::SCISSOR];
    let dynamic_state_create_info = vk::PipelineDynamicStateCreateInfo {
        s_type: vk::StructureType::PIPELINE_DYNAMIC_STATE_CREATE_INFO,
        p_next: ptr::null(),
        flags: vk::PipelineDynamicStateCreateFlags::empty(),
        dynamic_state_count: dynamic_states.len() as u32,
        p_dynamic_states: dynamic_states.as_ptr(),
        _marker: std::marker::PhantomData,
    };

    let viewport_state_create_info = vk::PipelineViewportStateCreateInfo {
        s_type: vk::StructureType::PIPELINE_VIEWPORT_STATE_CREATE_INFO,
        p_next: ptr::null(),
        flags: vk::PipelineViewportStateCreateFlags::empty(),
        scissor_count: 1, // Still need to specify count, but values are dynamic
        p_scissors: ptr::null(), // Dynamic, so no static values
        viewport_count: 1,
        p_viewports: ptr::null(), // Dynamic, so no static values
        _marker: std::marker::PhantomData,
    };

    let rasterization_statue_create_info = vk::PipelineRasterizationStateCreateInfo {
        s_type: vk::StructureType::PIPELINE_RASTERIZATION_STATE_CREATE_INFO,
        p_next: ptr::null(),
        flags: vk::PipelineRasterizationStateCreateFlags::empty(),
        depth_clamp_enable: vk::FALSE,
        cull_mode: vk::CullModeFlags::BACK,
        front_face: vk::FrontFace::CLOCKWISE,
        line_width: 1.0,
        polygon_mode: vk::PolygonMode::FILL,
        rasterizer_discard_enable: vk::FALSE,
        depth_bias_clamp: 0.0,
        depth_bias_constant_factor: 0.0,
        depth_bias_enable: vk::FALSE,
        depth_bias_slope_factor: 0.0,
        _marker: std::marker::PhantomData,
    };
    let multisample_state_create_info = vk::PipelineMultisampleStateCreateInfo {
        s_type: vk::StructureType::PIPELINE_MULTISAMPLE_STATE_CREATE_INFO,
        flags: vk::PipelineMultisampleStateCreateFlags::empty(),
        p_next: ptr::null(),
        rasterization_samples: vk::SampleCountFlags::TYPE_1,
        sample_shading_enable: vk::FALSE,
        min_sample_shading: 0.0,
        p_sample_mask: ptr::null(),
        alpha_to_one_enable: vk::FALSE,
        alpha_to_coverage_enable: vk::FALSE,
        _marker: std::marker::PhantomData,
    };

    let stencil_state = vk::StencilOpState {
        fail_op: vk::StencilOp::KEEP,
        pass_op: vk::StencilOp::KEEP,
        depth_fail_op: vk::StencilOp::KEEP,
        compare_op: vk::CompareOp::ALWAYS,
        compare_mask: 0,
        write_mask: 0,
        reference: 0,
    };

    let depth_state_create_info = vk::PipelineDepthStencilStateCreateInfo {
        s_type: vk::StructureType::PIPELINE_DEPTH_STENCIL_STATE_CREATE_INFO,
        p_next: ptr::null(),
        flags: vk::PipelineDepthStencilStateCreateFlags::empty(),
        depth_test_enable: vk::FALSE,
        depth_write_enable: vk::FALSE,
        depth_compare_op: vk::CompareOp::LESS_OR_EQUAL,
        depth_bounds_test_enable: vk::FALSE,
        stencil_test_enable: vk::FALSE,
        front: stencil_state,
        back: stencil_state,
        max_depth_bounds: 1.0,
        min_depth_bounds: 0.0,
        _marker: std::marker::PhantomData,
    };

    let color_blend_attachment_states = [vk::PipelineColorBlendAttachmentState {
        blend_enable: vk::FALSE,
        color_write_mask: vk::ColorComponentFlags::RGBA,
        src_color_blend_factor: vk::BlendFactor::ONE,
        dst_color_blend_factor: vk::BlendFactor::ZERO,
        color_blend_op: vk::BlendOp::ADD,
        src_alpha_blend_factor: vk::BlendFactor::ONE,
        dst_alpha_blend_factor: vk::BlendFactor::ZERO,
        alpha_blend_op: vk::BlendOp::ADD,
    }];

    let color_blend_state = vk::PipelineColorBlendStateCreateInfo {
        s_type: vk::StructureType::PIPELINE_COLOR_BLEND_STATE_CREATE_INFO,
        p_next: ptr::null(),
        flags: vk::PipelineColorBlendStateCreateFlags::empty(),
        logic_op_enable: vk::FALSE,
        logic_op: vk::LogicOp::COPY,
        attachment_count: color_blend_attachment_states.len() as u32,
        p_attachments: color_blend_attachment_states.as_ptr(),
        blend_constants: [0.0, 0.0, 0.0, 0.0],
        _marker: std::marker::PhantomData,
    };

    let pipeline_layout_create_info = vk::PipelineLayoutCreateInfo {
        s_type: vk::StructureType::PIPELINE_LAYOUT_CREATE_INFO,
        p_next: ptr::null(),
        flags: vk::PipelineLayoutCreateFlags::empty(),
        set_layout_count: 0,
        p_set_layouts: ptr::null(),
        push_constant_range_count: 0,
        p_push_constant_ranges: ptr::null(),
        _marker: std::marker::PhantomData,
    };

    let pipeline_layout = unsafe {
        device
            .create_pipeline_layout(&pipeline_layout_create_info, None)
            .expect("Failed to create pipeline layout!")
    };

    let graphic_pipeline_create_infos = [vk::GraphicsPipelineCreateInfo {
        s_type: vk::StructureType::GRAPHICS_PIPELINE_CREATE_INFO,
        p_next: ptr::null(),
        flags: vk::PipelineCreateFlags::empty(),
        stage_count: shader_stages.len() as u32,
        p_stages: shader_stages.as_ptr(),
        p_vertex_input_state: &vertex_input_state_create_info,
        p_input_assembly_state: &vertex_input_assembly_state_info,
        p_tessellation_state: ptr::null(),
        p_viewport_state: &viewport_state_create_info,
        p_rasterization_state: &rasterization_statue_create_info,
        p_multisample_state: &multisample_state_create_info,
        p_depth_stencil_state: &depth_state_create_info,
        p_color_blend_state: &color_blend_state,
        p_dynamic_state: &dynamic_state_create_info, // Add dynamic state info
        layout: pipeline_layout,
        render_pass,
        subpass: 0,
        base_pipeline_handle: vk::Pipeline::null(),
        base_pipeline_index: -1,
        _marker: std::marker::PhantomData,
    }];

    let graphics_pipelines = unsafe {
        device
            .create_graphics_pipelines(
                vk::PipelineCache::null(),
                &graphic_pipeline_create_infos,
                None,
            )
            .expect("Failed to create Graphics Pipeline!.")
    };

    unsafe {
        device.destroy_shader_module(vert_shader_module, None);
        device.destroy_shader_module(frag_shader_module, None);
    }

    (graphics_pipelines[0], pipeline_layout)
}

pub fn create_framebuffers(
    device: &ash::Device,
    render_pass: vk::RenderPass,
    image_views: &Vec<vk::ImageView>,
    swapchain_extent: vk::Extent2D,
) -> Vec<vk::Framebuffer> {
    let mut framebuffers = vec![];

    for &image_view in image_views.iter() {
        let attachments = [image_view];

        let framebuffer_create_info = vk::FramebufferCreateInfo {
            s_type: vk::StructureType::FRAMEBUFFER_CREATE_INFO,
            p_next: ptr::null(),
            flags: vk::FramebufferCreateFlags::empty(),
            render_pass,
            attachment_count: attachments.len() as u32,
            p_attachments: attachments.as_ptr(),
            width: swapchain_extent.width,
            height: swapchain_extent.height,
            layers: 1,
            _marker: std::marker::PhantomData,
        };

        let framebuffer = unsafe {
            device
                .create_framebuffer(&framebuffer_create_info, None)
                .expect("Failed to create Framebuffer!")
        };

        framebuffers.push(framebuffer);
    }

    framebuffers
}

pub fn create_command_pool(
    device: &ash::Device,
    queue_families: &QueueFamilyIndices,
) -> vk::CommandPool {
    let command_pool_create_info = vk::CommandPoolCreateInfo {
        s_type: vk::StructureType::COMMAND_POOL_CREATE_INFO,
        p_next: ptr::null(),
        flags: vk::CommandPoolCreateFlags::empty(),
        queue_family_index: queue_families.graphics_family.unwrap(),
        _marker: std::marker::PhantomData,
    };

    unsafe {
        device
            .create_command_pool(&command_pool_create_info, None)
            .expect("Failed to create Command Pool!")
    }
}

pub fn create_command_buffers(
    device: &ash::Device,
    command_pool: vk::CommandPool,
    graphics_pipeline: vk::Pipeline,
    framebuffers: &Vec<vk::Framebuffer>,
    render_pass: vk::RenderPass,
    surface_extent: vk::Extent2D,
) -> Vec<vk::CommandBuffer> {
    let command_buffer_allocate_info = vk::CommandBufferAllocateInfo {
        s_type: vk::StructureType::COMMAND_BUFFER_ALLOCATE_INFO,
        p_next: ptr::null(),
        command_buffer_count: framebuffers.len() as u32,
        command_pool,
        level: vk::CommandBufferLevel::PRIMARY,
        _marker: std::marker::PhantomData,
    };

    let command_buffers = unsafe {
        device
            .allocate_command_buffers(&command_buffer_allocate_info)
            .expect("Failed to allocate Command Buffers!")
    };

    for (i, &command_buffer) in command_buffers.iter().enumerate() {
        let command_buffer_begin_info = vk::CommandBufferBeginInfo {
            s_type: vk::StructureType::COMMAND_BUFFER_BEGIN_INFO,
            p_next: ptr::null(),
            p_inheritance_info: ptr::null(),
            flags: vk::CommandBufferUsageFlags::SIMULTANEOUS_USE,
            _marker: std::marker::PhantomData,
        };

        unsafe {
            device
                .begin_command_buffer(command_buffer, &command_buffer_begin_info)
                .expect("Failed to begin recording Command Buffer at beginning!");
        }

        let clear_values = [vk::ClearValue {
            color: vk::ClearColorValue {
                float32: [0.0, 0.0, 0.0, 1.0],
            },
        }];

        let render_pass_begin_info = vk::RenderPassBeginInfo {
            s_type: vk::StructureType::RENDER_PASS_BEGIN_INFO,
            p_next: ptr::null(),
            render_pass,
            framebuffer: framebuffers[i],
            render_area: vk::Rect2D {
                offset: vk::Offset2D { x: 0, y: 0 },
                extent: surface_extent,
            },
            clear_value_count: clear_values.len() as u32,
            p_clear_values: clear_values.as_ptr(),
            _marker: std::marker::PhantomData,
        };

        let viewports = [vk::Viewport {
            x: 0.0,
            y: 0.0,
            width: surface_extent.width as f32,
            height: surface_extent.height as f32,
            min_depth: 0.0,
            max_depth: 1.0,
        }];

        let scissors = [vk::Rect2D {
            offset: vk::Offset2D { x: 0, y: 0 },
            extent: surface_extent,
        }];

        unsafe {
            device.cmd_begin_render_pass(
                command_buffer,
                &render_pass_begin_info,
                vk::SubpassContents::INLINE,
            );
            device.cmd_bind_pipeline(
                command_buffer,
                vk::PipelineBindPoint::GRAPHICS,
                graphics_pipeline,
            );
            device.cmd_set_viewport(command_buffer, 0, &viewports);
            device.cmd_set_scissor(command_buffer, 0, &scissors);
            device.cmd_draw(command_buffer, 3, 1, 0, 0);
            device.cmd_end_render_pass(command_buffer);

            device
                .end_command_buffer(command_buffer)
                .expect("Failed to record Command Buffer at Ending!");
        }
    }

    command_buffers
}

pub fn create_shader_module(device: &ash::Device, wgsl_source: &str) -> vk::ShaderModule {
    // Step 1: Parse WGSL source into a Naga module
    let module = wgsl::parse_str(wgsl_source).expect("Failed to parse WGSL source");

    // Step 2: Validate the module (recommended for Vulkan compatibility)
    let mut validator = Validator::new(ValidationFlags::all(), Capabilities::all());
    let module_info = validator.validate(&module).expect("WGSL validation failed");

    // Step 3: Convert the module to SPIR-V
    let spv_options = spv::Options {
        flags: spv::WriterFlags::empty(), // Customize if needed (e.g., DEBUG)
        ..Default::default()
    };
    let spv_binary = spv::write_vec(&module, &module_info, &spv_options, None)
        .expect("Failed to convert WGSL to SPIR-V");

    // Step 4: Create the Vulkan shader module with the SPIR-V binary
    let shader_module_create_info = vk::ShaderModuleCreateInfo {
        s_type: vk::StructureType::SHADER_MODULE_CREATE_INFO,
        p_next: ptr::null(),
        flags: vk::ShaderModuleCreateFlags::empty(),
        code_size: spv_binary.len() * std::mem::size_of::<u32>(), // Size in bytes
        p_code: spv_binary.as_ptr(), // Pointer to SPIR-V binary (u32 words)
        _marker: std::marker::PhantomData,
    };

    unsafe {
        device
            .create_shader_module(&shader_module_create_info, None)
            .expect("Failed to create Shader Module!")
    }
}

impl RenderState {
    pub async fn new(
        window: Arc<Window>,
        event_loop: &winit::event_loop::ActiveEventLoop,
    ) -> RenderState {
        // init vulkan stuff
        let entry = unsafe { ash::Entry::load().unwrap() };
        let instance = create_instance(
            &entry,
            WINDOW_TITLE,
            VALIDATION.is_enable,
            &VALIDATION.required_validation_layers.to_vec(),
        );
        let surface_stuff = create_surface(
            &entry,
            &instance,
            window.clone(),
            WINDOW_WIDTH,
            WINDOW_HEIGHT,
        );
        let (debug_utils_loader, debug_merssager) =
            setup_debug_utils(VALIDATION.is_enable, &entry, &instance);
        let physical_device = pick_physical_device(&instance, &surface_stuff, &DEVICE_EXTENSIONS);
        let (device, family_indices) = create_logical_device(
            &instance,
            physical_device,
            &VALIDATION,
            &DEVICE_EXTENSIONS,
            &surface_stuff,
        );
        let graphics_queue =
            unsafe { device.get_device_queue(family_indices.graphics_family.unwrap(), 0) };
        let present_queue =
            unsafe { device.get_device_queue(family_indices.present_family.unwrap(), 0) };
        let swapchain_stuff = create_swapchain(
            &instance,
            &device,
            physical_device,
            &window,
            &surface_stuff,
            &family_indices,
        );
        let swapchain_imageviews = create_image_views(
            &device,
            swapchain_stuff.swapchain_format,
            &swapchain_stuff.swapchain_images,
        );
        let render_pass =
            RenderState::create_render_pass(&device, swapchain_stuff.swapchain_format);
        let (graphics_pipeline, pipeline_layout) =
            create_graphics_pipeline(&device, render_pass, swapchain_stuff.swapchain_extent);
        let swapchain_framebuffers = create_framebuffers(
            &device,
            render_pass,
            &swapchain_imageviews,
            swapchain_stuff.swapchain_extent,
        );
        let command_pool = create_command_pool(&device, &family_indices);
        let command_buffers = create_command_buffers(
            &device,
            command_pool,
            graphics_pipeline,
            &swapchain_framebuffers,
            render_pass,
            swapchain_stuff.swapchain_extent,
        );
        let sync_objects = RenderState::create_sync_objects(&device);

        let egui_ctx = Context::default();
        let egui_winit = State::new(
            egui_ctx.clone(),
            ViewportId::ROOT,
            &window,
            None,
            None,
            None,
        );
        let egui_renderer = Renderer::with_default_allocator(
            &instance,
            physical_device,
            device.clone(),
            render_pass,
            Options {
                srgb_framebuffer: true,
                ..Default::default()
            },
        )
        .unwrap();

        // cleanup(); the 'drop' function will take care of it.
        RenderState {
            window,
            // vulkan stuff
            _entry: entry,
            instance,
            surface: surface_stuff.surface,
            surface_loader: surface_stuff.surface_loader,
            debug_utils_loader,
            debug_merssager,

            _physical_device: physical_device,
            device,

            graphics_queue,
            present_queue,

            swapchain_loader: swapchain_stuff.swapchain_loader,
            swapchain: swapchain_stuff.swapchain,
            _swapchain_format: swapchain_stuff.swapchain_format,
            _swapchain_images: swapchain_stuff.swapchain_images,
            _swapchain_extent: swapchain_stuff.swapchain_extent,
            swapchain_imageviews,
            swapchain_framebuffers,

            pipeline_layout,
            render_pass,
            graphics_pipeline,

            command_pool,
            command_buffers,

            image_available_semaphores: sync_objects.image_available_semaphores,
            render_finished_semaphores: sync_objects.render_finished_semaphores,
            in_flight_fences: sync_objects.inflight_fences,
            current_frame: 0,

            egui_ctx,
            egui_winit,
            egui_renderer,
            textures_to_free: None,
        }
    }

    fn render(&mut self) {
        let wait_fences = [self.in_flight_fences[self.current_frame]];

        let (image_index, _is_sub_optimal) = unsafe {
            self.device
                .wait_for_fences(&wait_fences, true, std::u64::MAX)
                .expect("Failed to wait for Fence!");

            self.swapchain_loader
                .acquire_next_image(
                    self.swapchain,
                    std::u64::MAX,
                    self.image_available_semaphores[self.current_frame],
                    vk::Fence::null(),
                )
                .expect("Failed to acquire next image.")
        };

        let wait_semaphores = [self.image_available_semaphores[self.current_frame]];
        let wait_stages = [vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT];
        let signal_semaphores = [self.render_finished_semaphores[self.current_frame]];

        let submit_infos = [vk::SubmitInfo {
            s_type: vk::StructureType::SUBMIT_INFO,
            p_next: ptr::null(),
            wait_semaphore_count: wait_semaphores.len() as u32,
            p_wait_semaphores: wait_semaphores.as_ptr(),
            p_wait_dst_stage_mask: wait_stages.as_ptr(),
            command_buffer_count: 1,
            p_command_buffers: &self.command_buffers[image_index as usize],
            signal_semaphore_count: signal_semaphores.len() as u32,
            p_signal_semaphores: signal_semaphores.as_ptr(),
            _marker: std::marker::PhantomData,
        }];

        unsafe {
            self.device
                .reset_fences(&wait_fences)
                .expect("Failed to reset Fence!");

            self.device
                .queue_submit(
                    self.graphics_queue,
                    &submit_infos,
                    self.in_flight_fences[self.current_frame],
                )
                .expect("Failed to execute queue submit.");
        }

        let swapchains = [self.swapchain];

        let present_info = vk::PresentInfoKHR {
            s_type: vk::StructureType::PRESENT_INFO_KHR,
            p_next: ptr::null(),
            wait_semaphore_count: 1,
            p_wait_semaphores: signal_semaphores.as_ptr(),
            swapchain_count: 1,
            p_swapchains: swapchains.as_ptr(),
            p_image_indices: &image_index,
            p_results: ptr::null_mut(),
            _marker: std::marker::PhantomData,
        };

        unsafe {
            self.swapchain_loader
                .queue_present(self.present_queue, &present_info)
                .expect("Failed to execute queue present.");
        }

        self.current_frame = (self.current_frame + 1) % MAX_FRAMES_IN_FLIGHT;
    }

    pub fn resize(&mut self, new_width: u32, new_height: u32) {
        unsafe {
            // Wait for the device to finish all operations
            self.device
                .device_wait_idle()
                .expect("Failed to wait for device idle");

            // Clean up old resources
            self.cleanup_swapchain();

            // Update surface dimensions in surface_stuff (if needed elsewhere)
            let surface_stuff = SurfaceStuff {
                surface_loader: self.surface_loader.clone(),
                surface: self.surface,
                screen_width: new_width,
                screen_height: new_height,
            };

            // Recreate the swapchain
            let queue_family =
                find_queue_family(&self.instance, self._physical_device, &surface_stuff);
            let swapchain_stuff = create_swapchain(
                &self.instance,
                &self.device,
                self._physical_device,
                &self.window,
                &surface_stuff,
                &queue_family,
            );

            // Recreate image views
            let swapchain_imageviews = create_image_views(
                &self.device,
                swapchain_stuff.swapchain_format,
                &swapchain_stuff.swapchain_images,
            );

            // Recreate framebuffers
            let swapchain_framebuffers = create_framebuffers(
                &self.device,
                self.render_pass,
                &swapchain_imageviews,
                swapchain_stuff.swapchain_extent,
            );

            // Recreate command buffers (since they depend on framebuffers)
            let command_buffers = create_command_buffers(
                &self.device,
                self.command_pool,
                self.graphics_pipeline,
                &swapchain_framebuffers,
                self.render_pass,
                swapchain_stuff.swapchain_extent,
            );

            // Update RenderState with new resources
            self.swapchain_loader = swapchain_stuff.swapchain_loader;
            self.swapchain = swapchain_stuff.swapchain;
            self._swapchain_format = swapchain_stuff.swapchain_format;
            self._swapchain_images = swapchain_stuff.swapchain_images;
            self._swapchain_extent = swapchain_stuff.swapchain_extent;
            self.swapchain_imageviews = swapchain_imageviews;
            self.swapchain_framebuffers = swapchain_framebuffers;
            self.command_buffers = command_buffers;
        }
    }

    // Helper method to clean up old swapchain-related resources
    fn cleanup_swapchain(&mut self) {
        unsafe {
            // Destroy old framebuffers
            for &framebuffer in self.swapchain_framebuffers.iter() {
                self.device.destroy_framebuffer(framebuffer, None);
            }
            // Destroy old command buffers (free them back to the pool)
            self.device
                .free_command_buffers(self.command_pool, &self.command_buffers);
            // Destroy old image views
            for &image_view in self.swapchain_imageviews.iter() {
                self.device.destroy_image_view(image_view, None);
            }
            // Destroy old swapchain
            self.swapchain_loader
                .destroy_swapchain(self.swapchain, None);
        }
    }

    fn create_render_pass(device: &ash::Device, surface_format: vk::Format) -> vk::RenderPass {
        let color_attachment = vk::AttachmentDescription {
            format: surface_format,
            flags: vk::AttachmentDescriptionFlags::empty(),
            samples: vk::SampleCountFlags::TYPE_1,
            load_op: vk::AttachmentLoadOp::CLEAR,
            store_op: vk::AttachmentStoreOp::STORE,
            stencil_load_op: vk::AttachmentLoadOp::DONT_CARE,
            stencil_store_op: vk::AttachmentStoreOp::DONT_CARE,
            initial_layout: vk::ImageLayout::UNDEFINED,
            final_layout: vk::ImageLayout::PRESENT_SRC_KHR,
        };

        let color_attachment_ref = vk::AttachmentReference {
            attachment: 0,
            layout: vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL,
        };

        let subpasses = [vk::SubpassDescription {
            color_attachment_count: 1,
            p_color_attachments: &color_attachment_ref,
            p_depth_stencil_attachment: ptr::null(),
            flags: vk::SubpassDescriptionFlags::empty(),
            pipeline_bind_point: vk::PipelineBindPoint::GRAPHICS,
            input_attachment_count: 0,
            p_input_attachments: ptr::null(),
            p_resolve_attachments: ptr::null(),
            preserve_attachment_count: 0,
            p_preserve_attachments: ptr::null(),
            _marker: std::marker::PhantomData,
        }];

        let render_pass_attachments = [color_attachment];

        let subpass_dependencies = [vk::SubpassDependency {
            src_subpass: vk::SUBPASS_EXTERNAL,
            dst_subpass: 0,
            src_stage_mask: vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT,
            dst_stage_mask: vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT,
            src_access_mask: vk::AccessFlags::empty(),
            dst_access_mask: vk::AccessFlags::COLOR_ATTACHMENT_WRITE,
            dependency_flags: vk::DependencyFlags::empty(),
        }];

        let renderpass_create_info = vk::RenderPassCreateInfo {
            s_type: vk::StructureType::RENDER_PASS_CREATE_INFO,
            flags: vk::RenderPassCreateFlags::empty(),
            p_next: ptr::null(),
            attachment_count: render_pass_attachments.len() as u32,
            p_attachments: render_pass_attachments.as_ptr(),
            subpass_count: subpasses.len() as u32,
            p_subpasses: subpasses.as_ptr(),
            dependency_count: subpass_dependencies.len() as u32,
            p_dependencies: subpass_dependencies.as_ptr(),
            _marker: std::marker::PhantomData,
        };

        unsafe {
            device
                .create_render_pass(&renderpass_create_info, None)
                .expect("Failed to create render pass!")
        }
    }

    fn create_sync_objects(device: &ash::Device) -> SyncObjects {
        let mut sync_objects = SyncObjects {
            image_available_semaphores: vec![],
            render_finished_semaphores: vec![],
            inflight_fences: vec![],
        };

        let semaphore_create_info = vk::SemaphoreCreateInfo {
            s_type: vk::StructureType::SEMAPHORE_CREATE_INFO,
            p_next: ptr::null(),
            flags: vk::SemaphoreCreateFlags::empty(),
            _marker: std::marker::PhantomData,
        };

        let fence_create_info = vk::FenceCreateInfo {
            s_type: vk::StructureType::FENCE_CREATE_INFO,
            p_next: ptr::null(),
            flags: vk::FenceCreateFlags::SIGNALED,
            _marker: std::marker::PhantomData,
        };

        for _ in 0..MAX_FRAMES_IN_FLIGHT {
            unsafe {
                let image_available_semaphore = device
                    .create_semaphore(&semaphore_create_info, None)
                    .expect("Failed to create Semaphore Object!");
                let render_finished_semaphore = device
                    .create_semaphore(&semaphore_create_info, None)
                    .expect("Failed to create Semaphore Object!");
                let inflight_fence = device
                    .create_fence(&fence_create_info, None)
                    .expect("Failed to create Fence Object!");

                sync_objects
                    .image_available_semaphores
                    .push(image_available_semaphore);
                sync_objects
                    .render_finished_semaphores
                    .push(render_finished_semaphore);
                sync_objects.inflight_fences.push(inflight_fence);
            }
        }

        sync_objects
    }
}

impl Drop for RenderState {
    fn drop(&mut self) {
        unsafe {
            for i in 0..MAX_FRAMES_IN_FLIGHT {
                self.device
                    .destroy_semaphore(self.image_available_semaphores[i], None);
                self.device
                    .destroy_semaphore(self.render_finished_semaphores[i], None);
                self.device.destroy_fence(self.in_flight_fences[i], None);
            }

            self.device.destroy_command_pool(self.command_pool, None);

            for &framebuffer in self.swapchain_framebuffers.iter() {
                self.device.destroy_framebuffer(framebuffer, None);
            }

            self.device.destroy_pipeline(self.graphics_pipeline, None);
            self.device
                .destroy_pipeline_layout(self.pipeline_layout, None);
            self.device.destroy_render_pass(self.render_pass, None);

            for &imageview in self.swapchain_imageviews.iter() {
                self.device.destroy_image_view(imageview, None);
            }

            self.swapchain_loader
                .destroy_swapchain(self.swapchain, None);
            self.device.destroy_device(None);
            self.surface_loader.destroy_surface(self.surface, None);

            if VALIDATION.is_enable {
                self.debug_utils_loader
                    .destroy_debug_utils_messenger(self.debug_merssager, None);
            }
            self.instance.destroy_instance(None);
        }
    }
}

pub struct App {
    window: Option<Arc<Window>>,
    state: Option<RenderState>,
}

impl Default for App {
    fn default() -> Self {
        Self {
            window: Default::default(),
            state: None,
        }
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        info!("Creating window");
        if self.window.is_none() {
            let window_attributes =
                Window::default_attributes().with_title("Vulcan engine by Zuriefais");
            let window = Arc::new(event_loop.create_window(window_attributes).unwrap());
            self.window = Some(window.clone());

            let state = pollster::block_on(RenderState::new(window.clone(), event_loop));
            self.state = Some(state);
        }
    }

    fn window_event(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
        _: winit::window::WindowId,
        event: winit::event::WindowEvent,
    ) {
        match event {
            WindowEvent::CloseRequested
            | WindowEvent::KeyboardInput {
                event:
                    KeyEvent {
                        state: ElementState::Pressed,
                        physical_key: PhysicalKey::Code(KeyCode::Escape),
                        ..
                    },
                ..
            } => event_loop.exit(),
            WindowEvent::Resized(size) => {
                self.state.as_mut().unwrap().resize(size.width, size.height);
            }
            WindowEvent::RedrawRequested => {
                self.state.as_mut().unwrap().render();
                self.window.as_ref().unwrap().request_redraw();
            }
            _ => {}
        }
    }
}
