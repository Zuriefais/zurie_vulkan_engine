use crate::render::Renderer;
use fs::UBO;
use std::sync::Arc;
use vulkano::{
    buffer::{Buffer, BufferContents, BufferCreateInfo, BufferUsage, Subbuffer},
    command_buffer::{
        allocator::StandardCommandBufferAllocator, AutoCommandBufferBuilder,
        CommandBufferInheritanceInfo, CommandBufferUsage, SecondaryAutoCommandBuffer,
    },
    descriptor_set::{
        allocator::StandardDescriptorSetAllocator, PersistentDescriptorSet, WriteDescriptorSet,
    },
    device::Queue,
    image::{
        sampler::{Filter, Sampler, SamplerAddressMode, SamplerCreateInfo, SamplerMipmapMode},
        view::ImageView,
    },
    memory::allocator::{AllocationCreateInfo, MemoryTypeFilter},
    pipeline::{
        graphics::{
            color_blend::{
                AttachmentBlend, BlendFactor, BlendOp, ColorBlendAttachmentState, ColorBlendState,
            },
            input_assembly::InputAssemblyState,
            multisample::MultisampleState,
            rasterization::RasterizationState,
            vertex_input::{Vertex, VertexDefinition},
            viewport::{Viewport, ViewportState},
            GraphicsPipelineCreateInfo,
        },
        layout::PipelineDescriptorSetLayoutCreateInfo,
        DynamicState, GraphicsPipeline, Pipeline, PipelineBindPoint, PipelineLayout,
        PipelineShaderStageCreateInfo,
    },
    render_pass::Subpass,
};

/// Vertex for textured quads.
#[derive(BufferContents, Vertex)]
#[repr(C)]
pub struct TexturedVertex {
    #[format(R32G32_SFLOAT)]
    pub position: [f32; 2],
    #[format(R32G32_SFLOAT)]
    pub tex_coords: [f32; 2],
}

pub fn textured_quad(width: f32, height: f32) -> (Vec<TexturedVertex>, Vec<u32>) {
    (
        vec![
            TexturedVertex {
                position: [-(width / 2.0), -(height / 2.0)],
                tex_coords: [0.0, 1.0],
            },
            TexturedVertex {
                position: [-(width / 2.0), height / 2.0],
                tex_coords: [0.0, 0.0],
            },
            TexturedVertex {
                position: [width / 2.0, height / 2.0],
                tex_coords: [1.0, 0.0],
            },
            TexturedVertex {
                position: [width / 2.0, -(height / 2.0)],
                tex_coords: [1.0, 1.0],
            },
        ],
        vec![0, 2, 1, 0, 3, 2],
    )
}

/// A subpass pipeline that fills a quad over the frame.
pub struct ObjectDrawPipeline {
    gfx_queue: Arc<Queue>,
    subpass: Subpass,
    pipeline: Arc<GraphicsPipeline>,
    command_buffer_allocator: Arc<StandardCommandBufferAllocator>,
    descriptor_set_allocator: Arc<StandardDescriptorSetAllocator>,
    memory_allocator: Arc<
        vulkano::memory::allocator::GenericMemoryAllocator<
            vulkano::memory::allocator::FreeListAllocator,
        >,
    >,
    vertices: Subbuffer<[TexturedVertex]>,
    indices: Subbuffer<[u32]>,
}

impl ObjectDrawPipeline {
    pub fn new(app: &Renderer, subpass: Subpass) -> Self {
        let (vertices, indices) = textured_quad(2.0, 2.0);
        let memory_allocator = app.memory_allocator.clone();
        let vertex_buffer = Buffer::from_iter(
            memory_allocator.clone(),
            BufferCreateInfo {
                usage: BufferUsage::VERTEX_BUFFER,
                ..Default::default()
            },
            AllocationCreateInfo {
                memory_type_filter: MemoryTypeFilter::PREFER_DEVICE
                    | MemoryTypeFilter::HOST_SEQUENTIAL_WRITE,
                ..Default::default()
            },
            vertices,
        )
        .unwrap();
        let index_buffer = Buffer::from_iter(
            memory_allocator.clone(),
            BufferCreateInfo {
                usage: BufferUsage::INDEX_BUFFER,
                ..Default::default()
            },
            AllocationCreateInfo {
                memory_type_filter: MemoryTypeFilter::PREFER_DEVICE
                    | MemoryTypeFilter::HOST_SEQUENTIAL_WRITE,
                ..Default::default()
            },
            indices,
        )
        .unwrap();

        let pipeline = {
            let device = app.gfx_queue.device();
            let vs = vs::load(device.clone())
                .expect("failed to create shader module")
                .entry_point("main")
                .expect("shader entry point not found");
            let fs = fs::load(device.clone())
                .expect("failed to create shader module")
                .entry_point("main")
                .expect("shader entry point not found");
            let vertex_input_state = TexturedVertex::per_vertex()
                .definition(&vs.info().input_interface)
                .unwrap();
            let stages = [
                PipelineShaderStageCreateInfo::new(vs),
                PipelineShaderStageCreateInfo::new(fs),
            ];
            let layout = PipelineLayout::new(
                device.clone(),
                PipelineDescriptorSetLayoutCreateInfo::from_stages(&stages)
                    .into_pipeline_layout_create_info(device.clone())
                    .unwrap(),
            )
            .unwrap();

            GraphicsPipeline::new(
                device.clone(),
                None,
                GraphicsPipelineCreateInfo {
                    stages: stages.into_iter().collect(),
                    vertex_input_state: Some(vertex_input_state),
                    input_assembly_state: Some(InputAssemblyState::default()),
                    viewport_state: Some(ViewportState::default()),
                    rasterization_state: Some(RasterizationState::default()),
                    multisample_state: Some(MultisampleState::default()),
                    color_blend_state: Some(ColorBlendState::with_attachment_states(
                        subpass.num_color_attachments(),
                        ColorBlendAttachmentState {
                            blend: Some(AttachmentBlend {
                                src_color_blend_factor: BlendFactor::SrcAlpha, // Source color multiplied by its alpha
                                dst_color_blend_factor: BlendFactor::OneMinusSrcAlpha, // Destination color multiplied by (1 - source alpha)
                                color_blend_op: BlendOp::Add, // Add the two results together
                                src_alpha_blend_factor: BlendFactor::One, // Use the source alpha as-is
                                dst_alpha_blend_factor: BlendFactor::Zero, // Ignore the destination alpha
                                alpha_blend_op: BlendOp::Add, // Add the two results (effectively just keeping the source alpha)
                            }),
                            ..Default::default()
                        },
                    )),
                    dynamic_state: [DynamicState::Viewport].into_iter().collect(),
                    subpass: Some(subpass.clone().into()),
                    ..GraphicsPipelineCreateInfo::layout(layout)
                },
            )
            .unwrap()
        };
        let gfx_queue = app.gfx_queue();

        Self {
            gfx_queue,
            subpass,
            pipeline,
            command_buffer_allocator: app.command_buffer_allocator.clone(),
            descriptor_set_allocator: app.descriptor_set_allocator.clone(),
            memory_allocator,
            vertices: vertex_buffer,
            indices: index_buffer,
        }
    }

    fn create_image_sampler_nearest(
        &self,
        image: Arc<ImageView>,
        background_color: [f32; 4],
        camera: vs::Camera,
    ) -> Arc<PersistentDescriptorSet> {
        let layout = self.pipeline.layout().set_layouts().first().unwrap();
        let sampler = Sampler::new(
            self.gfx_queue.device().clone(),
            SamplerCreateInfo {
                mag_filter: Filter::Nearest,
                min_filter: Filter::Nearest,
                address_mode: [SamplerAddressMode::Repeat; 3],
                mipmap_mode: SamplerMipmapMode::Nearest,
                ..Default::default()
            },
        )
        .unwrap();

        let camera_buffer = Buffer::from_data(
            self.memory_allocator.clone(),
            BufferCreateInfo {
                usage: BufferUsage::UNIFORM_BUFFER,
                ..Default::default()
            },
            AllocationCreateInfo {
                memory_type_filter: MemoryTypeFilter::PREFER_DEVICE
                    | MemoryTypeFilter::HOST_SEQUENTIAL_WRITE,
                ..Default::default()
            },
            camera,
        )
        .unwrap();

        let ubo_buffer = Buffer::from_data(
            self.memory_allocator.clone(),
            BufferCreateInfo {
                usage: BufferUsage::UNIFORM_BUFFER,
                ..Default::default()
            },
            AllocationCreateInfo {
                memory_type_filter: MemoryTypeFilter::PREFER_DEVICE
                    | MemoryTypeFilter::HOST_SEQUENTIAL_WRITE,
                ..Default::default()
            },
            UBO { background_color },
        )
        .unwrap();

        PersistentDescriptorSet::new(
            &self.descriptor_set_allocator,
            layout.clone(),
            [
                WriteDescriptorSet::buffer(0, camera_buffer), // Camera uniform at binding 0
                WriteDescriptorSet::buffer(1, ubo_buffer),    // UBO at binding 1
                WriteDescriptorSet::sampler(2, sampler),      // Sampler at binding 2
                WriteDescriptorSet::image_view(3, image),     // Image view at binding 3
            ],
            [],
        )
        .unwrap()
    }

    /// Draws input `image` over a quad of size -1.0 to 1.0.
    pub fn draw(
        &self,
        viewport_dimensions: [u32; 2],
        image: Arc<ImageView>,
        background_color: [f32; 4],
        camera: vs::Camera,
    ) -> Arc<SecondaryAutoCommandBuffer> {
        let mut builder = AutoCommandBufferBuilder::secondary(
            self.command_buffer_allocator.as_ref(),
            self.gfx_queue.queue_family_index(),
            CommandBufferUsage::MultipleSubmit,
            CommandBufferInheritanceInfo {
                render_pass: Some(self.subpass.clone().into()),
                ..Default::default()
            },
        )
        .unwrap();
        let desc_set = self.create_image_sampler_nearest(image, background_color, camera);
        builder
            .set_viewport(
                0,
                [Viewport {
                    offset: [0.0, 0.0],
                    extent: [viewport_dimensions[0] as f32, viewport_dimensions[1] as f32],
                    depth_range: 0.0..=1.0,
                }]
                .into_iter()
                .collect(),
            )
            .unwrap()
            .bind_pipeline_graphics(self.pipeline.clone())
            .unwrap()
            .bind_descriptor_sets(
                PipelineBindPoint::Graphics,
                self.pipeline.layout().clone(),
                0,
                desc_set,
            )
            .unwrap()
            .bind_vertex_buffers(0, self.vertices.clone())
            .unwrap()
            .bind_index_buffer(self.indices.clone())
            .unwrap()
            .draw_indexed(self.indices.len() as u32, 1, 0, 0, 0)
            .unwrap();
        builder.build().unwrap()
    }
}

pub(crate) mod vs {
    vulkano_shaders::shader! {
        ty: "vertex",
        path: "shaders/object_draw/vs.glsl"
    }
}

mod fs {
    vulkano_shaders::shader! {
        ty: "fragment",
        path: "shaders/object_draw/fs.glsl"
    }
}
