use crate::camera::create_camera_buffer;
use crate::render::backend::Backend;
use crate::resources::resource_manager::ResourceManager;
use crate::utils::swapchain::create_image_view;
use crate::utils::vulkan_init::*;
use crate::vertex::{InstanceData, create_instance_buffer};
use ash::vk;
use std::ptr;

use zurie_render_glue::FrameContext;

pub struct ObjPass {
    pub pipeline: vk::Pipeline,
    pub pipeline_layout: vk::PipelineLayout,
    pub descriptor_set_layout: vk::DescriptorSetLayout,
    pub descriptor_sets: Vec<vk::DescriptorSet>,
    pub quad_buffer: vk::Buffer,
    pub instance_buffer: vk::Buffer,
    pub instance_buffer_memory: vk::DeviceMemory,
    pub camera_buffer: vk::Buffer,
    pub camera_buffer_memory: vk::DeviceMemory,
    pub descriptor_pool: vk::DescriptorPool,
    pub texture_image: vk::Image,
    pub texture_memory: vk::DeviceMemory,
    pub texture_view: vk::ImageView,
    pub sampler: vk::Sampler,
}

impl ObjPass {
    pub fn new(backend: &Backend, resources: &ResourceManager) -> anyhow::Result<Self> {
        let (pipeline, pipeline_layout, descriptor_set_layout) =
            create_graphics_pipeline(&backend.device, backend.swapchain_format);
        let descriptor_pool = create_descriptor_pool(&backend.device, 2)?;
        let quad_buffer = resources.quad_buffer();
        let (instance_buffer, instance_buffer_memory) = create_instance_buffer(
            &backend.instance,
            &backend.device,
            backend.physical_device,
            &backend.queue_family_indices,
            &[InstanceData::default()], // Empty initially
        );
        let camera = crate::camera::Camera {
            proj_mat: glam::Mat4::orthographic_rh(-1.0, 1.0, -1.0, 1.0, -1.0, 1.0),
        };
        let camera_buffer = create_camera_buffer(
            &backend.instance,
            &backend.device,
            backend.physical_device,
            &backend.queue_family_indices,
            camera,
        );
        let (_, camera_buffer_memory) = crate::camera::create_uniform_buffer(
            &backend.instance,
            &backend.device,
            backend.physical_device,
            &backend.queue_family_indices,
        );
        let (texture_image, texture_memory, texture_view) = create_texture_image(
            &backend.device,
            &backend.instance,
            backend.physical_device,
            &backend.queue_family_indices,
            backend.graphics_queue,
            backend.command_pool,
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
        let sampler = unsafe { backend.device.create_sampler(&sampler_info, None) }
            .expect("Failed to create sampler");
        let descriptor_sets = create_descriptor_sets(
            &backend.device,
            descriptor_pool,
            descriptor_set_layout,
            camera_buffer,
            texture_view,
            sampler,
        );

        Ok(Self {
            pipeline,
            pipeline_layout,
            descriptor_set_layout,
            descriptor_sets,
            quad_buffer,
            instance_buffer,
            instance_buffer_memory,
            camera_buffer,
            camera_buffer_memory,
            descriptor_pool,
            texture_image,
            texture_memory,
            texture_view,
            sampler,
        })
    }

    pub fn init(
        &mut self,
        backend: &Backend,
        swapchain_extent: vk::Extent2D,
    ) -> anyhow::Result<()> {
        // Already initialized in new; update if swapchain extent changes
        Ok(())
    }

    pub fn record(
        &self,
        command_buffer: vk::CommandBuffer,
        image_index: usize,
        backend: &Backend,
        resources: &ResourceManager,
        frame_context: &FrameContext,
        instance_data: Vec<InstanceData>,
    ) -> anyhow::Result<()> {
        let device = backend.device();
        let dynamic_rendering = ash::khr::dynamic_rendering::Device::new(&backend.instance, device);

        unsafe {
            let barrier = vk::ImageMemoryBarrier::default()
                .image(backend.swapchain_images[image_index])
                .old_layout(vk::ImageLayout::UNDEFINED)
                .new_layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL)
                .src_access_mask(vk::AccessFlags::empty())
                .dst_access_mask(vk::AccessFlags::COLOR_ATTACHMENT_WRITE)
                .subresource_range(
                    vk::ImageSubresourceRange::default()
                        .aspect_mask(vk::ImageAspectFlags::COLOR)
                        .level_count(1)
                        .layer_count(1),
                );
            device.cmd_pipeline_barrier(
                command_buffer,
                vk::PipelineStageFlags::TOP_OF_PIPE,
                vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT,
                vk::DependencyFlags::empty(),
                &[],
                &[],
                &[barrier],
            );

            let color_attachment = vk::RenderingAttachmentInfoKHR::default()
                .image_view(backend.swapchain_imageviews[image_index])
                .image_layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL)
                .load_op(vk::AttachmentLoadOp::CLEAR)
                .store_op(vk::AttachmentStoreOp::STORE)
                .clear_value(vk::ClearValue {
                    color: vk::ClearColorValue {
                        float32: frame_context.background_color,
                    },
                });

            let rendering_info = vk::RenderingInfoKHR::default()
                .render_area(vk::Rect2D {
                    offset: vk::Offset2D { x: 0, y: 0 },
                    extent: backend.swapchain_extent,
                })
                .layer_count(1)
                .color_attachments(std::slice::from_ref(&color_attachment));

            dynamic_rendering.cmd_begin_rendering(command_buffer, &rendering_info);

            device.cmd_bind_pipeline(
                command_buffer,
                vk::PipelineBindPoint::GRAPHICS,
                self.pipeline,
            );
            device.cmd_bind_descriptor_sets(
                command_buffer,
                vk::PipelineBindPoint::GRAPHICS,
                self.pipeline_layout,
                0,
                &self.descriptor_sets,
                &[],
            );

            device.cmd_bind_vertex_buffers(
                command_buffer,
                0,
                &[self.quad_buffer, self.instance_buffer],
                &[0, 0],
            );

            let viewport = vk::Viewport {
                x: 0.0,
                y: 0.0,
                width: backend.swapchain_extent.width as f32,
                height: backend.swapchain_extent.height as f32,
                min_depth: 0.0,
                max_depth: 1.0,
            };
            let scissor = vk::Rect2D {
                offset: vk::Offset2D { x: 0, y: 0 },
                extent: backend.swapchain_extent,
            };
            device.cmd_set_viewport(command_buffer, 0, &[viewport]);
            device.cmd_set_scissor(command_buffer, 0, &[scissor]);

            let instance_count = instance_data.len() as u32;
            device.cmd_draw(command_buffer, 4, instance_count, 0, 0);

            dynamic_rendering.cmd_end_rendering(command_buffer);
        }

        Ok(())
    }

    pub fn resize(&mut self, backend: &Backend, new_extent: vk::Extent2D) -> anyhow::Result<()> {
        let camera = crate::camera::Camera {
            proj_mat: glam::Mat4::orthographic_rh(-1.0, 1.0, -1.0, 1.0, -1.0, 1.0),
        };
        // Update existing camera buffer
        crate::camera::upload_camera_data(&backend.device, self.camera_buffer_memory, camera);
        unsafe {
            backend
                .device
                .free_descriptor_sets(self.descriptor_pool, &self.descriptor_sets)?;
        }
        self.descriptor_sets = create_descriptor_sets(
            &backend.device,
            self.descriptor_pool,
            self.descriptor_set_layout,
            self.camera_buffer,
            self.texture_view,
            self.sampler,
        );
        Ok(())
    }

    pub fn destroy(&mut self, backend: &Backend) {
        let device = backend.device();
        unsafe {
            device.destroy_pipeline(self.pipeline, None);
            device.destroy_pipeline_layout(self.pipeline_layout, None);
            device.destroy_descriptor_set_layout(self.descriptor_set_layout, None);
            device.destroy_descriptor_pool(self.descriptor_pool, None);
            device.destroy_buffer(self.camera_buffer, None);
            device.free_memory(self.camera_buffer_memory, None);
            device.destroy_buffer(self.instance_buffer, None);
            device.free_memory(self.instance_buffer_memory, None);
            device.destroy_image(self.texture_image, None);
            device.free_memory(self.texture_memory, None);
            device.destroy_image_view(self.texture_view, None);
            device.destroy_sampler(self.sampler, None);
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
