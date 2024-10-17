use crate::render::Renderer;

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
    memory::allocator::{AllocationCreateInfo, MemoryTypeFilter, StandardMemoryAllocator},
    pipeline::{
        graphics::{
            color_blend::{ColorBlendAttachmentState, ColorBlendState},
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
use zurie_types::Object;

#[derive(BufferContents, Vertex)]
#[repr(C)]
pub struct TriangleVertex {
    #[format(R32G32_SFLOAT)]
    pub vert_position: [f32; 2],
}

/// The vertex type that describes the unique data per instance.
#[derive(BufferContents, Vertex)]
#[repr(C)]
pub struct InstanceData {
    #[format(R32G32_SFLOAT)]
    position: [f32; 2],
    #[format(R32G32_SFLOAT)]
    scale: [f32; 2],
    #[format(R32G32B32A32_SFLOAT)]
    color: [f32; 4],
}

pub fn textured_quad() -> Vec<TriangleVertex> {
    vec![
        TriangleVertex {
            vert_position: [1.0, -(1.0)],
        },
        TriangleVertex {
            vert_position: [-(1.0), 1.0],
        },
        TriangleVertex {
            vert_position: [1.0, 1.0],
        },
        TriangleVertex {
            vert_position: [1.0, -(1.0)],
        },
    ]
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
    vertices: Subbuffer<[TriangleVertex]>,
}

impl ObjectDrawPipeline {
    pub fn new(app: &Renderer, subpass: Subpass) -> Self {
        let memory_allocator = Arc::new(StandardMemoryAllocator::new_default(app.device.clone()));
        let command_buffer_allocator = app.command_buffer_allocator.clone();

        let vertices = [
            // First triangle
            TriangleVertex {
                vert_position: [-0.5, -0.5],
            },
            TriangleVertex {
                vert_position: [0.5, -0.5],
            },
            TriangleVertex {
                vert_position: [-0.5, 0.5],
            },
            // Second triangle
            TriangleVertex {
                vert_position: [0.5, -0.5],
            },
            TriangleVertex {
                vert_position: [0.5, 0.5],
            },
            TriangleVertex {
                vert_position: [-0.5, 0.5],
            },
        ];
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

        let pipeline = {
            let vs = vs::load(app.device.clone())
                .unwrap()
                .entry_point("main")
                .unwrap();
            let fs = fs::load(app.device.clone())
                .unwrap()
                .entry_point("main")
                .unwrap();
            let vertex_input_state = [TriangleVertex::per_vertex(), InstanceData::per_instance()]
                .definition(&vs.info().input_interface)
                .unwrap();
            let stages = [
                PipelineShaderStageCreateInfo::new(vs),
                PipelineShaderStageCreateInfo::new(fs),
            ];
            let layout = PipelineLayout::new(
                app.device.clone(),
                PipelineDescriptorSetLayoutCreateInfo::from_stages(&stages)
                    .into_pipeline_layout_create_info(app.device.clone())
                    .unwrap(),
            )
            .unwrap();

            GraphicsPipeline::new(
                app.device.clone(),
                None,
                GraphicsPipelineCreateInfo {
                    stages: stages.into_iter().collect(),
                    // Use the implementations of the `Vertex` trait to describe to vulkano how the two vertex
                    // types are expected to be used.
                    vertex_input_state: Some(vertex_input_state),
                    input_assembly_state: Some(InputAssemblyState::default()),
                    viewport_state: Some(ViewportState::default()),
                    rasterization_state: Some(RasterizationState::default()),
                    multisample_state: Some(MultisampleState::default()),
                    color_blend_state: Some(ColorBlendState::with_attachment_states(
                        subpass.num_color_attachments(),
                        ColorBlendAttachmentState::default(),
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
            command_buffer_allocator,
            descriptor_set_allocator: app.descriptor_set_allocator.clone(),
            memory_allocator,
            vertices: vertex_buffer,
        }
    }

    fn create_descriptor(&self, camera: vs::Camera) -> Arc<PersistentDescriptorSet> {
        let layout = self
            .pipeline
            .layout()
            .set_layouts()
            .first()
            .expect("No set layout found");

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
        .expect("Failed to create camera buffer");

        PersistentDescriptorSet::new(
            &self.descriptor_set_allocator,
            layout.clone(),
            [WriteDescriptorSet::buffer(0, camera_buffer)],
            [],
        )
        .expect("Failed to create descriptor set")
    }

    /// Draws input `image` over a quad of size -1.0 to 1.0.
    pub fn draw(
        &self,
        viewport_dimensions: [u32; 2],
        camera: vs::Camera,
        objects: &[Object],
    ) -> Arc<SecondaryAutoCommandBuffer> {
        let mut builder = AutoCommandBufferBuilder::secondary(
            self.command_buffer_allocator.as_ref(),
            self.gfx_queue.queue_family_index(),
            CommandBufferUsage::OneTimeSubmit,
            CommandBufferInheritanceInfo {
                render_pass: Some(self.subpass.clone().into()),
                ..Default::default()
            },
        )
        .unwrap();
        let desc_set = self.create_descriptor(camera);
        let instance_data: Vec<InstanceData> = objects
            .iter()
            .map(|obj| InstanceData {
                position: obj.position.into(),
                scale: obj.scale.into(),
                color: obj.color.into(),
            })
            .collect();

        let instance_buffer = Buffer::from_iter(
            self.memory_allocator.clone(),
            BufferCreateInfo {
                usage: BufferUsage::VERTEX_BUFFER,
                ..Default::default()
            },
            AllocationCreateInfo {
                memory_type_filter: MemoryTypeFilter::PREFER_DEVICE
                    | MemoryTypeFilter::HOST_SEQUENTIAL_WRITE,
                ..Default::default()
            },
            instance_data,
        )
        .unwrap();
        let instance_buffer_len = instance_buffer.len();
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
            .bind_vertex_buffers(0, (self.vertices.clone(), instance_buffer))
            .unwrap()
            .draw(self.vertices.len() as u32, instance_buffer_len as u32, 0, 0)
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
