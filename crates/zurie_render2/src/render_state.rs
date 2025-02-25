use crate::constants::*;
use crate::debug;
use crate::debug::setup_debug_utils;
use crate::platforms;
use crate::structures::*;
use crate::tools;
use crate::utils::*;
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
use zurie_render_glue::RenderBackend;
// Constants
const WINDOW_TITLE: &'static str = "15.Hello Triangle";
const MAX_FRAMES_IN_FLIGHT: usize = 2;

struct SyncObjects {
    image_available_semaphores: Vec<vk::Semaphore>,
    render_finished_semaphores: Vec<vk::Semaphore>,
    inflight_fences: Vec<vk::Fence>,
}

impl RenderBackend for RenderState {
    fn init(window: Arc<Window>, event_loop: &ActiveEventLoop) -> Result<Self, anyhow::Error> {
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
        egui_ctx.set_style(gruvbox_egui::gruvbox_dark_theme());
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
        Ok(RenderState {
            window,
            // vulkan stuff
            entry: entry,
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
        })
    }

    fn render(
        &mut self,
        background_color: [f32; 4],
        camera: &zurie_types::camera::Camera,
        objects: Arc<std::sync::RwLock<Vec<zurie_types::Object>>>,
    ) -> anyhow::Result<()> {
        todo!()
    }

    fn handle_window_event(&mut self, event: &winit::event::WindowEvent) -> anyhow::Result<()> {
        self.egui_winit.on_window_event(&self.window, &event);
        Ok(())
    }

    fn get_egui_context(&self) -> egui::Context {
        self.egui_ctx.clone()
    }

    fn resize_window(&mut self, size: (u32, u32)) -> anyhow::Result<()> {
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
                screen_width: size.0,
                screen_height: size.1,
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
            self.swapchain_format = swapchain_stuff.swapchain_format;
            self.swapchain_images = swapchain_stuff.swapchain_images;
            self.swapchain_extent = swapchain_stuff.swapchain_extent;
            self.swapchain_imageviews = swapchain_imageviews;
            self.swapchain_framebuffers = swapchain_framebuffers;
            self.command_buffers = command_buffers;
        }
        Ok(())
    }
}

struct RenderState {
    window: Arc<Window>,
    // vulkan stuff
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

impl RenderState {
    fn render(&mut self) {
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

            // Submit and present logic...
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
    }

    fn record_command_buffer(
        &mut self,
        image_index: usize,
        clipped_primitives: &[ClippedPrimitive],
        pixels_per_point: f32,
    ) {
        let command_buffer = self.command_buffers[image_index];
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

            let render_pass_begin_info = vk::RenderPassBeginInfo {
                s_type: vk::StructureType::RENDER_PASS_BEGIN_INFO,
                p_next: ptr::null(),
                render_pass: self.render_pass,
                framebuffer: self.swapchain_framebuffers[image_index],
                render_area: vk::Rect2D {
                    offset: vk::Offset2D { x: 0, y: 0 },
                    extent: self.swapchain_extent,
                },
                clear_value_count: 1,
                p_clear_values: &vk::ClearValue {
                    color: vk::ClearColorValue {
                        float32: [0.0, 0.0, 0.0, 1.0],
                    },
                },
                _marker: std::marker::PhantomData,
            };

            self.device.cmd_begin_render_pass(
                command_buffer,
                &render_pass_begin_info,
                vk::SubpassContents::INLINE,
            );
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

            self.device.cmd_end_render_pass(command_buffer);
            self.device
                .end_command_buffer(command_buffer)
                .expect("Failed to end command buffer");
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

            let state = RenderState::init(window.clone(), event_loop).unwrap();
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
                state.render();
                self.window.as_ref().unwrap().request_redraw();
            }
            _ => {}
        }
    }
}
