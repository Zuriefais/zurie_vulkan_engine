use crate::constants::*;
use crate::debug;
use crate::debug::setup_debug_utils;
use crate::platforms;
use crate::structures::*;
use crate::tools;
use crate::utils::*;
use anyhow::Ok;
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
use zurie_render_glue::FrameContext;
use zurie_render_glue::RenderBackend;
use zurie_render_glue::RenderConfig;
use zurie_types::Object;
// Constants
const WINDOW_TITLE: &'static str = "15.Hello Triangle";
const MAX_FRAMES_IN_FLIGHT: usize = 2;

struct SyncObjects {
    image_available_semaphores: Vec<vk::Semaphore>,
    render_finished_semaphores: Vec<vk::Semaphore>,
    inflight_fences: Vec<vk::Fence>,
}

impl RenderBackend for RenderState {
    fn init(config: RenderConfig) -> Result<Self, anyhow::Error> {
        let entry = unsafe { ash::Entry::load().unwrap() };
        let window = config.window;
        let instance = create_instance(
            &entry,
            WINDOW_TITLE,
            VALIDATION.is_enable,
            &VALIDATION.required_validation_layers.to_vec(),
        )?;
        let surface_stuff = create_surface(
            &entry,
            &instance,
            window.clone(),
            WINDOW_WIDTH,
            WINDOW_HEIGHT,
        );
        let (debug_utils_loader, debug_merssager) =
            setup_debug_utils(VALIDATION.is_enable, &entry, &instance);
        let physical_device = pick_physical_device(&instance, &surface_stuff)?;

        let (device, family_indices) =
            create_logical_device(&instance, physical_device, &VALIDATION, &surface_stuff);
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

        info!("Swapchain format: {:?}", swapchain_stuff.swapchain_format);
        let (graphics_pipeline, pipeline_layout) = create_graphics_pipeline(
            &device,
            swapchain_stuff.swapchain_extent,
            swapchain_stuff.swapchain_format,
        );
        let command_pool = create_command_pool(&device, &family_indices);
        let command_buffers = create_command_buffers_dynamic(
            &device,
            command_pool,
            graphics_pipeline,
            &swapchain_imageviews,
            swapchain_stuff.swapchain_extent,
            swapchain_stuff.swapchain_format,
        );
        let sync_objects = RenderState::create_sync_objects(&device);

        let egui_winit = State::new(
            config.egui_context.clone(),
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
            egui_ash_renderer::DynamicRendering {
                color_attachment_format: swapchain_stuff.swapchain_format,
                depth_attachment_format: None,
            },
            Options {
                srgb_framebuffer: true,
                ..Default::default()
            },
        )
        .unwrap();

        Ok(RenderState {
            window,
            entry,
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
            swapchain_format: swapchain_stuff.swapchain_format,
            swapchain_images: swapchain_stuff.swapchain_images,
            swapchain_extent: swapchain_stuff.swapchain_extent,
            swapchain_imageviews,
            pipeline_layout,
            graphics_pipeline,
            command_pool,
            command_buffers,
            image_available_semaphores: sync_objects.image_available_semaphores,
            render_finished_semaphores: sync_objects.render_finished_semaphores,
            in_flight_fences: sync_objects.inflight_fences,
            current_frame: 0,
            egui_ctx: config.egui_context,
            egui_winit,
            egui_renderer,
            textures_to_free: None,
        })
    }

    fn render<I>(&mut self, context: FrameContext, objects: I) -> anyhow::Result<()>
    where
        I: Iterator<Item = Object>,
    {
        let wait_fences = [self.in_flight_fences[self.current_frame]];

        unsafe {
            self.device
                .wait_for_fences(&wait_fences, true, std::u64::MAX)
                .expect("Failed to wait for Fence!");
            let (image_index, _is_sub_optimal) = self
                .swapchain_loader
                .acquire_next_image(
                    self.swapchain,
                    std::u64::MAX,
                    self.image_available_semaphores[self.current_frame],
                    vk::Fence::null(),
                )
                .expect("Failed to acquire next image.");

            if let Some(textures) = self.textures_to_free.take() {
                self.egui_renderer
                    .free_textures(&textures)
                    .expect("Failed to free textures");
            }

            let scale_factor = self.window.scale_factor() as f32;
            self.egui_ctx.set_pixels_per_point(scale_factor);
            let raw_input = self.egui_winit.take_egui_input(&self.window);
            let egui::FullOutput {
                platform_output,
                textures_delta,
                shapes,
                pixels_per_point,
                ..
            } = self.egui_ctx.run(raw_input, |ctx| {
                egui::Window::new("Hello Triangle UI").show(ctx, |ui| {
                    ui.label("This is a triangle rendered with Vulkan and egui overlay!");
                });
            });

            self.egui_winit
                .handle_platform_output(&self.window, platform_output);

            if !textures_delta.free.is_empty() {
                self.textures_to_free = Some(textures_delta.free.clone());
            }

            if !textures_delta.set.is_empty() {
                self.egui_renderer
                    .set_textures(
                        self.graphics_queue,
                        self.command_pool,
                        textures_delta.set.as_slice(),
                    )
                    .expect("Failed to update texture");
            }

            let clipped_primitives = self.egui_ctx.tessellate(shapes, pixels_per_point);
            self.device
                .device_wait_idle()
                .expect("Failed to wait for device idle");
            self.device
                .reset_command_pool(self.command_pool, vk::CommandPoolResetFlags::empty())
                .expect("Failed to reset command pool");

            self.record_command_buffer(image_index as usize, &clipped_primitives, pixels_per_point);

            let wait_semaphores = [self.image_available_semaphores[self.current_frame]];
            let wait_stages = [vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT];
            let signal_semaphores = [self.render_finished_semaphores[self.current_frame]];
            let command_buffers = [self.command_buffers[image_index as usize]];
            let submit_infos = [vk::SubmitInfo {
                s_type: vk::StructureType::SUBMIT_INFO,
                p_next: ptr::null(),
                wait_semaphore_count: wait_semaphores.len() as u32,
                p_wait_semaphores: wait_semaphores.as_ptr(),
                p_wait_dst_stage_mask: wait_stages.as_ptr(),
                command_buffer_count: 1,
                p_command_buffers: command_buffers.as_ptr(),
                signal_semaphore_count: signal_semaphores.len() as u32,
                p_signal_semaphores: signal_semaphores.as_ptr(),
                _marker: std::marker::PhantomData,
            }];

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

            self.swapchain_loader
                .queue_present(self.present_queue, &present_info)
                .expect("Failed to execute queue present.");
        }

        self.current_frame = (self.current_frame + 1) % MAX_FRAMES_IN_FLIGHT;
        Ok(())
    }

    fn handle_window_event(&mut self, event: &winit::event::WindowEvent) -> anyhow::Result<()> {
        self.egui_winit.on_window_event(&self.window, &event);
        Ok(())
    }

    fn resize_window(&mut self, size: (u32, u32)) -> anyhow::Result<()> {
        unsafe {
            self.device
                .device_wait_idle()
                .expect("Failed to wait for device idle");
            self.cleanup_swapchain();

            let surface_stuff = SurfaceStuff {
                surface_loader: self.surface_loader.clone(),
                surface: self.surface,
                screen_width: size.0,
                screen_height: size.1,
            };

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

            let swapchain_imageviews = create_image_views(
                &self.device,
                swapchain_stuff.swapchain_format,
                &swapchain_stuff.swapchain_images,
            );

            let command_buffers = create_command_buffers_dynamic(
                &self.device,
                self.command_pool,
                self.graphics_pipeline,
                &swapchain_imageviews,
                swapchain_stuff.swapchain_extent,
                swapchain_stuff.swapchain_format,
            );

            self.swapchain_loader = swapchain_stuff.swapchain_loader;
            self.swapchain = swapchain_stuff.swapchain;
            self.swapchain_format = swapchain_stuff.swapchain_format;
            self.swapchain_images = swapchain_stuff.swapchain_images;
            self.swapchain_extent = swapchain_stuff.swapchain_extent;
            self.swapchain_imageviews = swapchain_imageviews;
            self.command_buffers = command_buffers;
        }
        Ok(())
    }
}

struct RenderState {
    window: Arc<Window>,
    entry: ash::Entry,
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
    swapchain_images: Vec<vk::Image>,
    swapchain_format: vk::Format,
    swapchain_extent: vk::Extent2D,
    swapchain_imageviews: Vec<vk::ImageView>,
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

impl RenderState {
    fn record_command_buffer(
        &mut self,
        image_index: usize,
        clipped_primitives: &[ClippedPrimitive],
        pixels_per_point: f32,
    ) {
        let command_buffer = self.command_buffers[image_index];
        let dynamic_rendering =
            ash::khr::dynamic_rendering::Device::new(&self.instance, &self.device);

        unsafe {
            self.device
                .begin_command_buffer(command_buffer, &vk::CommandBufferBeginInfo {
                    s_type: vk::StructureType::COMMAND_BUFFER_BEGIN_INFO,
                    p_next: ptr::null(),
                    flags: vk::CommandBufferUsageFlags::SIMULTANEOUS_USE,
                    p_inheritance_info: ptr::null(),
                    _marker: std::marker::PhantomData,
                })
                .expect("Failed to begin command buffer");

            // Transition swapchain image to COLOR_ATTACHMENT_OPTIMAL
            let barrier = vk::ImageMemoryBarrier {
                s_type: vk::StructureType::IMAGE_MEMORY_BARRIER,
                p_next: ptr::null(),
                src_access_mask: vk::AccessFlags::empty(),
                dst_access_mask: vk::AccessFlags::COLOR_ATTACHMENT_WRITE,
                old_layout: vk::ImageLayout::UNDEFINED,
                new_layout: vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL,
                src_queue_family_index: vk::QUEUE_FAMILY_IGNORED,
                dst_queue_family_index: vk::QUEUE_FAMILY_IGNORED,
                image: self.swapchain_images[image_index],
                subresource_range: vk::ImageSubresourceRange {
                    aspect_mask: vk::ImageAspectFlags::COLOR,
                    base_mip_level: 0,
                    level_count: 1,
                    base_array_layer: 0,
                    layer_count: 1,
                },
                _marker: std::marker::PhantomData,
            };
            self.device.cmd_pipeline_barrier(
                command_buffer,
                vk::PipelineStageFlags::TOP_OF_PIPE,
                vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT,
                vk::DependencyFlags::empty(),
                &[],
                &[],
                &[barrier],
            );

            let color_attachment = vk::RenderingAttachmentInfoKHR {
                s_type: vk::StructureType::RENDERING_ATTACHMENT_INFO_KHR,
                p_next: ptr::null(),
                image_view: self.swapchain_imageviews[image_index],
                image_layout: vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL,
                resolve_mode: vk::ResolveModeFlags::NONE,
                resolve_image_view: vk::ImageView::null(),
                resolve_image_layout: vk::ImageLayout::UNDEFINED,
                load_op: vk::AttachmentLoadOp::CLEAR,
                store_op: vk::AttachmentStoreOp::STORE,
                clear_value: vk::ClearValue {
                    color: vk::ClearColorValue {
                        float32: [0.0, 0.0, 0.0, 1.0],
                    },
                },
                _marker: std::marker::PhantomData,
            };

            let rendering_info = vk::RenderingInfoKHR {
                s_type: vk::StructureType::RENDERING_INFO_KHR,
                p_next: ptr::null(),
                flags: vk::RenderingFlagsKHR::empty(),
                render_area: vk::Rect2D {
                    offset: vk::Offset2D { x: 0, y: 0 },
                    extent: self.swapchain_extent,
                },
                layer_count: 1,
                view_mask: 0,
                color_attachment_count: 1,
                p_color_attachments: &color_attachment,
                p_depth_attachment: ptr::null(),
                p_stencil_attachment: ptr::null(),
                _marker: std::marker::PhantomData,
            };

            dynamic_rendering.cmd_begin_rendering(command_buffer, &rendering_info);

            self.device.cmd_bind_pipeline(
                command_buffer,
                vk::PipelineBindPoint::GRAPHICS,
                self.graphics_pipeline,
            );

            let viewport = vk::Viewport {
                x: 0.0,
                y: 0.0,
                width: self.swapchain_extent.width as f32,
                height: self.swapchain_extent.height as f32,
                min_depth: 0.0,
                max_depth: 1.0,
            };
            self.device.cmd_set_viewport(command_buffer, 0, &[viewport]);

            let scissor = vk::Rect2D {
                offset: vk::Offset2D { x: 0, y: 0 },
                extent: self.swapchain_extent,
            };
            self.device.cmd_set_scissor(command_buffer, 0, &[scissor]);

            self.device.cmd_draw(command_buffer, 3, 1, 0, 0);
            self.egui_renderer
                .cmd_draw(
                    command_buffer,
                    self.swapchain_extent,
                    pixels_per_point,
                    clipped_primitives,
                )
                .expect("Failed to draw egui primitives");

            dynamic_rendering.cmd_end_rendering(command_buffer);

            // Transition swapchain image to PRESENT_SRC_KHR
            let barrier = vk::ImageMemoryBarrier {
                s_type: vk::StructureType::IMAGE_MEMORY_BARRIER,
                p_next: ptr::null(),
                src_access_mask: vk::AccessFlags::COLOR_ATTACHMENT_WRITE,
                dst_access_mask: vk::AccessFlags::empty(),
                old_layout: vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL,
                new_layout: vk::ImageLayout::PRESENT_SRC_KHR,
                src_queue_family_index: vk::QUEUE_FAMILY_IGNORED,
                dst_queue_family_index: vk::QUEUE_FAMILY_IGNORED,
                image: self.swapchain_images[image_index],
                subresource_range: vk::ImageSubresourceRange {
                    aspect_mask: vk::ImageAspectFlags::COLOR,
                    base_mip_level: 0,
                    level_count: 1,
                    base_array_layer: 0,
                    layer_count: 1,
                },
                _marker: std::marker::PhantomData,
            };
            self.device.cmd_pipeline_barrier(
                command_buffer,
                vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT,
                vk::PipelineStageFlags::BOTTOM_OF_PIPE,
                vk::DependencyFlags::empty(),
                &[],
                &[],
                &[barrier],
            );

            self.device
                .end_command_buffer(command_buffer)
                .expect("Failed to end command buffer");
        }
    }

    fn cleanup_swapchain(&mut self) {
        unsafe {
            self.device
                .free_command_buffers(self.command_pool, &self.command_buffers);
            for &image_view in self.swapchain_imageviews.iter() {
                self.device.destroy_image_view(image_view, None);
            }
            self.swapchain_loader
                .destroy_swapchain(self.swapchain, None);
        }
    }

    fn create_sync_objects(device: &ash::Device) -> SyncObjects {
        // Same as before
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
            self.device.destroy_pipeline(self.graphics_pipeline, None);
            self.device
                .destroy_pipeline_layout(self.pipeline_layout, None);
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
fn create_command_buffers_dynamic(
    device: &ash::Device,
    command_pool: vk::CommandPool,
    pipeline: vk::Pipeline,
    image_views: &[vk::ImageView],
    extent: vk::Extent2D,
    format: vk::Format,
) -> Vec<vk::CommandBuffer> {
    let command_buffer_allocate_info = vk::CommandBufferAllocateInfo {
        s_type: vk::StructureType::COMMAND_BUFFER_ALLOCATE_INFO,
        p_next: ptr::null(),
        command_pool,
        level: vk::CommandBufferLevel::PRIMARY,
        command_buffer_count: image_views.len() as u32,
        _marker: std::marker::PhantomData,
    };

    let command_buffers = unsafe {
        device
            .allocate_command_buffers(&command_buffer_allocate_info)
            .expect("Failed to allocate command buffers")
    };

    command_buffers
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
            let egui_context = Context::default();
            egui_context.set_style(gruvbox_egui::gruvbox_dark_theme());
            let state = RenderState::init(RenderConfig {
                window: window.clone(),
                event_loop,
                egui_context,
            })
            .unwrap();
            self.state = Some(state);
        }
    }

    fn window_event(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
        _: winit::window::WindowId,
        event: winit::event::WindowEvent,
    ) {
        let state = self.state.as_mut().unwrap();
        state.handle_window_event(&event);
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
                state.resize_window((size.width, size.height));
            }
            WindowEvent::ScaleFactorChanged { scale_factor, .. } => {
                // Update egui's scale factor immediately
                state.egui_ctx.set_pixels_per_point(scale_factor as f32);
                log::info!("Scale factor: {}", scale_factor);
            }
            WindowEvent::RedrawRequested => {
                state
                    .render(Default::default(), Vec::new().into_iter())
                    .unwrap();
                self.window.as_ref().unwrap().request_redraw();
            }
            _ => {}
        }
    }
}
