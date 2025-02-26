use crate::constants::*;
use crate::debug;
use crate::debug::setup_debug_utils;
use crate::platforms;
use crate::structures::*;
use crate::tools;
use anyhow::anyhow;
use ash::khr::swapchain;
use ash::vk;
use ash::vk::DeviceCreateInfo;
use egui::{ClippedPrimitive, Context, TextureId, ViewportId};
use egui_ash_renderer::{Options, Renderer};
use egui_winit::State;
use log::info;
use naga::back::spv; // For generating SPIR-V
use naga::front::wgsl; // For parsing WGSL
use naga::valid::{Capabilities, ValidationFlags, Validator};
use std::ffi::CStr;
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
) -> anyhow::Result<ash::Instance> {
    if is_enable_debug
        && debug::check_validation_layer_support(entry, required_validation_layers) == false
    {
        panic!("Validation layers requested, but not available!");
    }

    let app_name = CString::new(window_title).unwrap();
    let engine_name = CString::new("Vulkan Engine").unwrap();
    let api_version = unsafe { entry.enumerate_instance_version().unwrap() };
    if api_version < vk::make_api_version(0, 1, 3, 0) {
        Err(anyhow!("Vulkan 1.3 is not supported by this system!"))?
    }
    let app_info = vk::ApplicationInfo {
        p_application_name: app_name.as_ptr(),
        s_type: vk::StructureType::APPLICATION_INFO,
        p_next: ptr::null(),
        application_version: APPLICATION_VERSION,
        p_engine_name: engine_name.as_ptr(),
        engine_version: ENGINE_VERSION,
        api_version,
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

    Ok(instance)
}

pub fn pick_physical_device(
    instance: &ash::Instance,
    surface_stuff: &SurfaceStuff,
) -> anyhow::Result<vk::PhysicalDevice> {
    let devices = {
        let mut devices = unsafe { instance.enumerate_physical_devices()? };
        devices.sort_by_key(|device| {
            let props = unsafe { instance.get_physical_device_properties(*device) };
            match props.device_type {
                vk::PhysicalDeviceType::DISCRETE_GPU => 0,
                vk::PhysicalDeviceType::INTEGRATED_GPU => 1,
                _ => 2,
            }
        });
        devices
    };

    let device = devices.into_iter().find(|device| {
        let device = *device;

        // Does device supports graphics and present queues
        let props = unsafe { instance.get_physical_device_queue_family_properties(device) };
        for (index, family) in props.iter().filter(|f| f.queue_count > 0).enumerate() {
            let index = index as u32;

            let present_support = unsafe {
                surface_stuff
                    .surface_loader
                    .get_physical_device_surface_support(device, index, surface_stuff.surface)
                    .expect("Failed to get device surface support")
            };
        }

        // Does device support desired extensions
        let extension_props = unsafe {
            instance
                .enumerate_device_extension_properties(device)
                .expect("Failed to get device ext properties")
        };
        let extention_support = extension_props.iter().any(|ext| {
            let name = unsafe { CStr::from_ptr(ext.extension_name.as_ptr()) };
            swapchain::NAME == name
        });

        // Does the device have available formats for the given surface
        let formats = unsafe {
            surface_stuff
                .surface_loader
                .get_physical_device_surface_formats(device, surface_stuff.surface)
                .expect("Failed to get physical device surface formats")
        };

        // Does the device have available present modes for the given surface
        let present_modes = unsafe {
            surface_stuff
                .surface_loader
                .get_physical_device_surface_present_modes(device, surface_stuff.surface)
                .expect("Failed to get physical device surface present modes")
        };

        // Check 1.3 features
        let mut features13 = vk::PhysicalDeviceVulkan13Features::default();
        let mut features = vk::PhysicalDeviceFeatures2::default().push_next(&mut features13);
        unsafe { instance.get_physical_device_features2(device, &mut features) };

        extention_support
            && !formats.is_empty()
            && !present_modes.is_empty()
            && features13.dynamic_rendering == vk::TRUE
            && features13.synchronization2 == vk::TRUE
    });

    match device {
        Some(device) => Ok(device),
        None => Err(anyhow!("Failed to find a suitable GPU!")),
    }
}

pub fn create_logical_device(
    instance: &ash::Instance,
    physical_device: vk::PhysicalDevice,
    validation: &debug::ValidationInfo,
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

    let device_extensions_ptrs = [
        swapchain::NAME.as_ptr(),
        ash::khr::dynamic_rendering::NAME.as_ptr(),
        #[cfg(any(target_os = "macos", target_os = "ios"))]
        ash::khr::portability_subset::NAME.as_ptr(),
    ];

    let mut features13 = vk::PhysicalDeviceVulkan13Features::default()
        .dynamic_rendering(true)
        .synchronization2(true);
    let mut features = vk::PhysicalDeviceFeatures2::default().push_next(&mut features13);

    let device_create_info = vk::DeviceCreateInfo::default()
        .queue_create_infos(&queue_create_infos)
        .enabled_extension_names(&device_extensions_ptrs)
        .push_next(&mut features);

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
    swapchain_extent: vk::Extent2D,
    swapchain_format: vk::Format,
) -> (vk::Pipeline, vk::PipelineLayout) {
    let vert_shader_module = create_shader_module(device, include_str!("shaders/vert.wgsl"));
    let frag_shader_module = create_shader_module(device, include_str!("shaders/frag.wgsl"));

    let main_function_name = std::ffi::CString::new("main").unwrap();

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
        scissor_count: 1,
        p_scissors: ptr::null(),
        viewport_count: 1,
        p_viewports: ptr::null(),
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

    // Allocate VkPipelineRenderingCreateInfo on the heap to ensure stable memory
    let color_attachment_formats = [swapchain_format];
    let pipeline_rendering_create_info = Box::new(vk::PipelineRenderingCreateInfo {
        s_type: vk::StructureType::PIPELINE_RENDERING_CREATE_INFO,
        p_next: ptr::null(),
        view_mask: 0,
        color_attachment_count: color_attachment_formats.len() as u32,
        p_color_attachment_formats: color_attachment_formats.as_ptr(),
        depth_attachment_format: vk::Format::UNDEFINED,
        stencil_attachment_format: vk::Format::UNDEFINED,
        _marker: std::marker::PhantomData,
    });

    let graphic_pipeline_create_infos = [vk::GraphicsPipelineCreateInfo {
        s_type: vk::StructureType::GRAPHICS_PIPELINE_CREATE_INFO,
        p_next: Box::into_raw(pipeline_rendering_create_info) as *const std::ffi::c_void,
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
        p_dynamic_state: &dynamic_state_create_info,
        layout: pipeline_layout,
        render_pass: vk::RenderPass::null(),
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
            .expect("Failed to create Graphics Pipeline!")
    };

    // Clean up shader modules
    unsafe {
        device.destroy_shader_module(vert_shader_module, None);
        device.destroy_shader_module(frag_shader_module, None);
        // Free the boxed pipeline_rendering_create_info after use
        let _ = Box::from_raw(
            graphic_pipeline_create_infos[0].p_next as *mut vk::PipelineRenderingCreateInfo,
        );
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
