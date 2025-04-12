use crate::camera::Camera;
use crate::camera::create_camera_buffer;
use crate::constants::*;
use crate::debug::setup_debug_utils;
use crate::structures::*;
use crate::utils::vulkan_init::*;

use crate::utils::swapchain::*;
use crate::vertex::InstanceData;
use crate::vertex::create_quad_buffer;
use anyhow::Ok;
use ash::vk;
use ash::vk::Handle;
use egui::{ClippedPrimitive, Context, TextureId, ViewportId};
use egui_ash_renderer::{Options, Renderer};
use egui_winit::State;
use glam::Mat4;
use glam::Vec2;
use glam::Vec4;
use log::info;

use std::ptr;
use std::sync::Arc;
use winit::application::ApplicationHandler;
use winit::event::KeyEvent;

use winit::event::{ElementState, WindowEvent};

use winit::event_loop::ActiveEventLoop;

use winit::keyboard::{KeyCode, PhysicalKey};

use winit::window::Window;

use zurie_render_glue::FrameContext;
use zurie_render_glue::RenderBackend;
use zurie_render_glue::RenderConfig;
use zurie_types::Object;

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
            create_logical_device(&instance, physical_device, &surface_stuff);
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
        let (graphics_pipeline, pipeline_layout, descriptor_set_layout) =
            create_graphics_pipeline(&device, swapchain_stuff.swapchain_format);
        let command_pool = create_command_pool(&device, &family_indices);
        let descriptor_pool = create_descriptor_pool(&device, 2);
        let command_buffers =
            create_command_buffers_dynamic(&device, command_pool, &swapchain_imageviews);
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

        let queue_family_vec = vec![
            family_indices.graphics_family.unwrap(),
            family_indices.present_family.unwrap(),
        ];

        let quad_buffer =
            create_quad_buffer(&instance, &device, physical_device, &queue_family_vec);
        let (_, quad_buffer_memory) = crate::vertex::create_vertex_buffer(
            &instance,
            &device,
            physical_device,
            &queue_family_vec,
        );

        let camera = Camera {
            proj_mat: Mat4::orthographic_rh(
                -1.0, 1.0, -1.0, 1.0, -1.0, 1.0, // Left, right, bottom, top, near, far
            ),
            cam_pos: Vec2::ZERO,
        };
        let camera_buffer = create_camera_buffer(
            &instance,
            &device,
            physical_device,
            &queue_family_vec,
            camera,
        );
        let (_, camera_buffer_memory) = crate::camera::create_uniform_buffer(
            &instance,
            &device,
            physical_device,
            &queue_family_vec,
        );

        let (texture_image, texture_memory, texture_view) = create_texture_image(
            &device,
            &instance,
            physical_device,
            &queue_family_vec,
            graphics_queue,
            command_pool,
        );

        let sampler_info = vk::SamplerCreateInfo {
            s_type: vk::StructureType::SAMPLER_CREATE_INFO,
            p_next: ptr::null(),
            flags: vk::SamplerCreateFlags::empty(),
            mag_filter: vk::Filter::LINEAR,
            min_filter: vk::Filter::LINEAR,
            mipmap_mode: vk::SamplerMipmapMode::LINEAR,
            address_mode_u: vk::SamplerAddressMode::REPEAT,
            address_mode_v: vk::SamplerAddressMode::REPEAT,
            address_mode_w: vk::SamplerAddressMode::REPEAT,
            mip_lod_bias: 0.0,
            anisotropy_enable: vk::FALSE,
            max_anisotropy: 1.0,
            compare_enable: vk::FALSE,
            compare_op: vk::CompareOp::ALWAYS,
            min_lod: 0.0,
            max_lod: 0.0,
            border_color: vk::BorderColor::INT_OPAQUE_BLACK,
            unnormalized_coordinates: vk::FALSE,
            _marker: std::marker::PhantomData,
        };
        let sampler = unsafe { device.create_sampler(&sampler_info, None) }
            .expect("Failed to create sampler");

        let descriptor_sets = create_descriptor_sets(
            &device,
            descriptor_pool,
            descriptor_set_layout,
            camera_buffer,
            texture_view,
            sampler,
        );

        let instance_data = vec![InstanceData {
            position: Vec2::new(0.0, 0.0),
            scale: Vec2::new(1.0, 1.0),
            color: Vec4::new(1.0, 1.0, 0.0, 1.0),
        }];

        let (instance_buffer, instance_buffer_memory) = crate::vertex::create_instance_buffer(
            &instance,
            &device,
            physical_device,
            &queue_family_vec,
            &instance_data,
        );

        Ok(RenderState {
            window,
            entry,
            instance,
            surface: surface_stuff.surface,
            surface_loader: surface_stuff.surface_loader,
            debug_utils_loader,
            debug_merssager,
            physical_device,
            device,
            graphics_queue,
            present_queue,
            swapchain_loader: swapchain_stuff.swapchain_loader,
            swapchain: swapchain_stuff.swapchain,
            swapchain_format: swapchain_stuff.swapchain_format,
            swapchain_images: swapchain_stuff.swapchain_images,
            swapchain_extent: swapchain_stuff.swapchain_extent,
            swapchain_imageviews,
            queue_family_indices: swapchain_stuff.queue_family_indices,
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
            descriptor_pool,
            descriptor_set_layout,
            quad_buffer,
            quad_buffer_memory,
            camera_buffer,
            camera_buffer_memory,
            texture_image,
            texture_memory,
            texture_view,
            sampler,
            descriptor_sets,
            instance_buffer,
            instance_buffer_memory,
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

            self.record_command_buffer(
                image_index as usize,
                &clipped_primitives,
                pixels_per_point,
                self.quad_buffer, // Use stored buffer
            );

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
        let _ = self.egui_winit.on_window_event(&self.window, &event);
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
                find_queue_family(&self.instance, self.physical_device, &surface_stuff);
            let swapchain_stuff = create_swapchain(
                &self.instance,
                &self.device,
                self.physical_device,
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
                &swapchain_imageviews,
            );

            // Update the projection matrix
            let aspect_ratio = size.0 as f32 / size.1 as f32;
            let camera = Camera {
                proj_mat: Mat4::orthographic_rh(
                    -aspect_ratio, // Left
                    aspect_ratio,  // Right
                    -1.0,          // Bottom
                    1.0,           // Top
                    -1.0,          // Near
                    1.0,           // Far
                ),
                cam_pos: Vec2::ZERO,
            };

            // Use queue family indices from queue_family
            let queue_family_vec = if queue_family.graphics_family == queue_family.present_family {
                vec![queue_family.graphics_family.unwrap()]
            } else {
                vec![
                    queue_family.graphics_family.unwrap(),
                    queue_family.present_family.unwrap(),
                ]
            };

            self.camera_buffer = create_camera_buffer(
                &self.instance,
                &self.device,
                self.physical_device,
                &queue_family_vec,
                camera,
            );

            // Reset the descriptor pool to free old descriptor sets
            self.device.reset_descriptor_pool(
                self.descriptor_pool,
                vk::DescriptorPoolResetFlags::empty(),
            )?;

            // Update descriptor sets
            self.descriptor_sets = create_descriptor_sets(
                &self.device,
                self.descriptor_pool,
                self.descriptor_set_layout,
                self.camera_buffer,
                self.texture_view,
                self.sampler,
            );

            self.swapchain_loader = swapchain_stuff.swapchain_loader;
            self.swapchain = swapchain_stuff.swapchain;
            self.swapchain_format = swapchain_stuff.swapchain_format;
            self.swapchain_images = swapchain_stuff.swapchain_images;
            self.swapchain_extent = swapchain_stuff.swapchain_extent;
            self.swapchain_imageviews = swapchain_imageviews;
            self.command_buffers = command_buffers;
            self.queue_family_indices = swapchain_stuff.queue_family_indices;
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
    physical_device: vk::PhysicalDevice,
    device: ash::Device,
    graphics_queue: vk::Queue,
    present_queue: vk::Queue,
    swapchain_loader: ash::khr::swapchain::Device,
    swapchain: vk::SwapchainKHR,
    swapchain_images: Vec<vk::Image>,
    swapchain_format: vk::Format,
    swapchain_extent: vk::Extent2D,
    swapchain_imageviews: Vec<vk::ImageView>,
    queue_family_indices: Vec<u32>,
    pipeline_layout: vk::PipelineLayout,
    graphics_pipeline: vk::Pipeline,
    command_pool: vk::CommandPool,
    command_buffers: Vec<vk::CommandBuffer>,
    image_available_semaphores: Vec<vk::Semaphore>,
    render_finished_semaphores: Vec<vk::Semaphore>,
    in_flight_fences: Vec<vk::Fence>,
    current_frame: usize,
    descriptor_pool: vk::DescriptorPool,
    descriptor_set_layout: vk::DescriptorSetLayout,
    egui_ctx: Context,
    egui_winit: State,
    egui_renderer: Renderer,
    textures_to_free: Option<Vec<TextureId>>,
    quad_buffer: vk::Buffer,
    quad_buffer_memory: vk::DeviceMemory,
    camera_buffer: vk::Buffer,
    camera_buffer_memory: vk::DeviceMemory,
    texture_image: vk::Image,
    texture_memory: vk::DeviceMemory,
    texture_view: vk::ImageView,
    sampler: vk::Sampler,
    descriptor_sets: Vec<vk::DescriptorSet>,
    instance_buffer: vk::Buffer,
    instance_buffer_memory: vk::DeviceMemory,
}

impl RenderState {
    fn record_command_buffer(
        &mut self,
        image_index: usize,
        clipped_primitives: &[ClippedPrimitive],
        pixels_per_point: f32,
        quad_buffer: vk::Buffer,
    ) {
        let command_buffer = self.command_buffers[image_index];
        let dynamic_rendering =
            ash::khr::dynamic_rendering::Device::new(&self.instance, &self.device);

        unsafe {
            self.device
                .begin_command_buffer(
                    command_buffer,
                    &vk::CommandBufferBeginInfo {
                        s_type: vk::StructureType::COMMAND_BUFFER_BEGIN_INFO,
                        p_next: ptr::null(),
                        flags: vk::CommandBufferUsageFlags::SIMULTANEOUS_USE,
                        p_inheritance_info: ptr::null(),
                        _marker: std::marker::PhantomData,
                    },
                )
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
            self.device.cmd_bind_descriptor_sets(
                command_buffer,
                vk::PipelineBindPoint::GRAPHICS,
                self.pipeline_layout,
                0,
                &self.descriptor_sets[..], // Reuse stored descriptor sets
                &[],
            );
            self.device.cmd_bind_vertex_buffers(
                command_buffer,
                0,
                &[self.quad_buffer, self.instance_buffer],
                &[0, 0],
            );
            let scissor = vk::Rect2D {
                offset: vk::Offset2D { x: 0, y: 0 },
                extent: self.swapchain_extent,
            };
            self.device.cmd_set_scissor(command_buffer, 0, &[scissor]);

            self.device.cmd_draw(command_buffer, 4, 1, 0, 0);
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
            // Free command buffers
            if !self.command_buffers.is_empty() {
                self.device
                    .free_command_buffers(self.command_pool, &self.command_buffers);
                self.command_buffers.clear();
            }

            // Destroy image views
            if !self.swapchain_imageviews.is_empty() {
                for &image_view in self.swapchain_imageviews.iter() {
                    if !image_view.is_null() {
                        self.device.destroy_image_view(image_view, None);
                    }
                }
                self.swapchain_imageviews.clear();
            }

            // Destroy swapchain
            if !self.swapchain.is_null() {
                self.swapchain_loader
                    .destroy_swapchain(self.swapchain, None);
                self.swapchain = vk::SwapchainKHR::null();
            }
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

impl Drop for RenderState {
    fn drop(&mut self) {
        unsafe {
            self.device
                .device_wait_idle()
                .expect("Failed to wait for device idle");

            // Destroy synchronization objects
            for i in 0..MAX_FRAMES_IN_FLIGHT {
                self.device
                    .destroy_semaphore(self.image_available_semaphores[i], None);
                self.device
                    .destroy_semaphore(self.render_finished_semaphores[i], None);
                self.device.destroy_fence(self.in_flight_fences[i], None);
            }

            // Clean up command pool and buffers
            self.device.destroy_command_pool(self.command_pool, None);

            // Clean up pipeline and layout
            self.device.destroy_pipeline(self.graphics_pipeline, None);
            self.device
                .destroy_pipeline_layout(self.pipeline_layout, None);

            // Clean up swapchain resources
            for &imageview in self.swapchain_imageviews.iter() {
                self.device.destroy_image_view(imageview, None);
            }
            self.swapchain_loader
                .destroy_swapchain(self.swapchain, None);

            // Clean up descriptor-related resources
            self.device
                .destroy_descriptor_pool(self.descriptor_pool, None);
            self.device
                .destroy_descriptor_set_layout(self.descriptor_set_layout, None);

            // Clean up buffers and memory
            self.device.destroy_buffer(self.quad_buffer, None);
            self.device.free_memory(self.quad_buffer_memory, None);
            self.device.destroy_buffer(self.camera_buffer, None);
            self.device.free_memory(self.camera_buffer_memory, None);

            // Clean up texture resources
            self.device.destroy_image(self.texture_image, None);
            self.device.free_memory(self.texture_memory, None);
            self.device.destroy_image_view(self.texture_view, None);
            self.device.destroy_sampler(self.sampler, None);

            // Destroy device and instance
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

fn create_texture_image(
    device: &ash::Device,
    instance: &ash::Instance,
    physical_device: vk::PhysicalDevice,
    queue_family_indices: &[u32],
    queue: vk::Queue,
    command_pool: vk::CommandPool,
) -> (vk::Image, vk::DeviceMemory, vk::ImageView) {
    // Create staging buffer
    let pixel_data = [255u8, 255, 255, 255]; // White pixel (RGBA)
    let buffer_size = pixel_data.len() as vk::DeviceSize;
    let staging_buffer_info = vk::BufferCreateInfo {
        s_type: vk::StructureType::BUFFER_CREATE_INFO,
        p_next: ptr::null(),
        flags: vk::BufferCreateFlags::empty(),
        size: buffer_size,
        usage: vk::BufferUsageFlags::TRANSFER_SRC,
        sharing_mode: vk::SharingMode::EXCLUSIVE,
        queue_family_index_count: queue_family_indices.len() as u32,
        p_queue_family_indices: queue_family_indices.as_ptr(),
        ..Default::default()
    };
    let staging_buffer = unsafe { device.create_buffer(&staging_buffer_info, None).unwrap() };
    let staging_mem_requirements = unsafe { device.get_buffer_memory_requirements(staging_buffer) };

    // Allocate staging buffer memory (host-visible)
    let memory_properties =
        unsafe { instance.get_physical_device_memory_properties(physical_device) };
    let memory_type_index = memory_properties
        .memory_types
        .iter()
        .enumerate()
        .find(|(i, mem_type)| {
            let type_filter = staging_mem_requirements.memory_type_bits & (1 << i);
            let properties =
                vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT;
            type_filter != 0 && mem_type.property_flags.contains(properties)
        })
        .map(|(i, _)| i as u32)
        .expect("No suitable memory type for staging buffer");

    let staging_memory_info = vk::MemoryAllocateInfo {
        s_type: vk::StructureType::MEMORY_ALLOCATE_INFO,
        p_next: ptr::null(),
        allocation_size: staging_mem_requirements.size,
        memory_type_index,
        ..Default::default()
    };
    let staging_memory = unsafe { device.allocate_memory(&staging_memory_info, None).unwrap() };
    unsafe {
        device
            .bind_buffer_memory(staging_buffer, staging_memory, 0)
            .unwrap()
    };

    // Upload pixel data to staging buffer
    unsafe {
        let data_ptr = device
            .map_memory(staging_memory, 0, buffer_size, vk::MemoryMapFlags::empty())
            .expect("Failed to map staging memory") as *mut u8;
        ptr::copy_nonoverlapping(pixel_data.as_ptr(), data_ptr, pixel_data.len());
        device.unmap_memory(staging_memory);
    }

    // Create texture image
    let image_info = vk::ImageCreateInfo {
        s_type: vk::StructureType::IMAGE_CREATE_INFO,
        p_next: ptr::null(),
        flags: vk::ImageCreateFlags::empty(), // Fixed: Use ImageCreateFlags
        image_type: vk::ImageType::TYPE_2D,
        format: vk::Format::R8G8B8A8_SRGB,
        extent: vk::Extent3D {
            width: 1,
            height: 1,
            depth: 1,
        },
        mip_levels: 1,
        array_layers: 1,
        samples: vk::SampleCountFlags::TYPE_1,
        tiling: vk::ImageTiling::OPTIMAL,
        usage: vk::ImageUsageFlags::SAMPLED | vk::ImageUsageFlags::TRANSFER_DST,
        sharing_mode: vk::SharingMode::EXCLUSIVE,
        queue_family_index_count: queue_family_indices.len() as u32,
        p_queue_family_indices: queue_family_indices.as_ptr(),
        initial_layout: vk::ImageLayout::UNDEFINED,
        ..Default::default()
    };
    let image = unsafe { device.create_image(&image_info, None).unwrap() };
    let mem_requirements = unsafe { device.get_image_memory_requirements(image) };

    // Allocate texture image memory (device-local)
    let memory_type_index = memory_properties
        .memory_types
        .iter()
        .enumerate()
        .find(|(i, mem_type)| {
            mem_type
                .property_flags
                .contains(vk::MemoryPropertyFlags::DEVICE_LOCAL)
        })
        .map(|(i, _)| i as u32)
        .expect("No suitable memory type for texture image");

    let memory = allocate_buffer_memory(
        instance,
        device,
        physical_device,
        &mem_requirements,
        vk::MemoryPropertyFlags::DEVICE_LOCAL,
    );
    unsafe { device.bind_image_memory(image, memory, 0).unwrap() };

    // Transfer pixel data to image
    let command_buffer = begin_single_time_commands(device, command_pool);
    let barrier = vk::ImageMemoryBarrier {
        s_type: vk::StructureType::IMAGE_MEMORY_BARRIER,
        p_next: ptr::null(),
        src_access_mask: vk::AccessFlags::empty(),
        dst_access_mask: vk::AccessFlags::TRANSFER_WRITE,
        old_layout: vk::ImageLayout::UNDEFINED,
        new_layout: vk::ImageLayout::TRANSFER_DST_OPTIMAL,
        src_queue_family_index: vk::QUEUE_FAMILY_IGNORED,
        dst_queue_family_index: vk::QUEUE_FAMILY_IGNORED,
        image,
        subresource_range: vk::ImageSubresourceRange {
            aspect_mask: vk::ImageAspectFlags::COLOR,
            base_mip_level: 0,
            level_count: 1,
            base_array_layer: 0,
            layer_count: 1,
        },
        ..Default::default()
    };
    unsafe {
        device.cmd_pipeline_barrier(
            command_buffer,
            vk::PipelineStageFlags::TOP_OF_PIPE,
            vk::PipelineStageFlags::TRANSFER,
            vk::DependencyFlags::empty(),
            &[],
            &[],
            &[barrier],
        );

        let copy_region = vk::BufferImageCopy {
            buffer_offset: 0,
            buffer_row_length: 0,
            buffer_image_height: 0,
            image_subresource: vk::ImageSubresourceLayers {
                aspect_mask: vk::ImageAspectFlags::COLOR,
                mip_level: 0,
                base_array_layer: 0,
                layer_count: 1,
            },
            image_offset: vk::Offset3D { x: 0, y: 0, z: 0 },
            image_extent: vk::Extent3D {
                width: 1,
                height: 1,
                depth: 1,
            },
            ..Default::default()
        };
        device.cmd_copy_buffer_to_image(
            command_buffer,
            staging_buffer,
            image,
            vk::ImageLayout::TRANSFER_DST_OPTIMAL,
            &[copy_region],
        );

        let barrier = vk::ImageMemoryBarrier {
            src_access_mask: vk::AccessFlags::TRANSFER_WRITE,
            dst_access_mask: vk::AccessFlags::SHADER_READ,
            old_layout: vk::ImageLayout::TRANSFER_DST_OPTIMAL,
            new_layout: vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
            ..barrier
        };
        device.cmd_pipeline_barrier(
            command_buffer,
            vk::PipelineStageFlags::TRANSFER,
            vk::PipelineStageFlags::FRAGMENT_SHADER,
            vk::DependencyFlags::empty(),
            &[],
            &[],
            &[barrier],
        );
    }
    end_single_time_commands(device, command_pool, queue, command_buffer);

    // Clean up staging buffer
    unsafe {
        device.destroy_buffer(staging_buffer, None);
        device.free_memory(staging_memory, None);
    }

    // Create image view
    let view = create_image_view(
        device,
        image,
        vk::Format::R8G8B8A8_SRGB,
        vk::ImageAspectFlags::COLOR,
        1,
    );

    (image, memory, view)
}

// Add these helper functions
fn begin_single_time_commands(
    device: &ash::Device,
    command_pool: vk::CommandPool,
) -> vk::CommandBuffer {
    let alloc_info = vk::CommandBufferAllocateInfo {
        s_type: vk::StructureType::COMMAND_BUFFER_ALLOCATE_INFO,
        p_next: std::ptr::null(),
        level: vk::CommandBufferLevel::PRIMARY,
        command_pool,
        command_buffer_count: 1,
        _marker: std::marker::PhantomData,
    };

    let command_buffer = unsafe { device.allocate_command_buffers(&alloc_info).unwrap()[0] };
    let begin_info = vk::CommandBufferBeginInfo {
        s_type: vk::StructureType::COMMAND_BUFFER_BEGIN_INFO,
        p_next: std::ptr::null(),
        flags: vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT,
        p_inheritance_info: std::ptr::null(),
        _marker: std::marker::PhantomData,
    };

    unsafe {
        device
            .begin_command_buffer(command_buffer, &begin_info)
            .unwrap()
    };
    command_buffer
}

fn end_single_time_commands(
    device: &ash::Device,
    command_pool: vk::CommandPool,
    queue: vk::Queue,
    command_buffer: vk::CommandBuffer,
) {
    unsafe { device.end_command_buffer(command_buffer).unwrap() };

    let submit_info = vk::SubmitInfo {
        s_type: vk::StructureType::SUBMIT_INFO,
        p_next: std::ptr::null(),
        wait_semaphore_count: 0,
        p_wait_semaphores: std::ptr::null(),
        p_wait_dst_stage_mask: std::ptr::null(),
        command_buffer_count: 1,
        p_command_buffers: &command_buffer,
        signal_semaphore_count: 0,
        p_signal_semaphores: std::ptr::null(),
        _marker: std::marker::PhantomData,
    };

    unsafe {
        device
            .queue_submit(queue, &[submit_info], vk::Fence::null())
            .unwrap();
        device.queue_wait_idle(queue).unwrap();
        device.free_command_buffers(command_pool, &[command_buffer]);
    }
}

fn create_command_buffers_dynamic(
    device: &ash::Device,
    command_pool: vk::CommandPool,
    image_views: &[vk::ImageView],
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
