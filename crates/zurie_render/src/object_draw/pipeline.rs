use crate::{render::Renderer, sprite::SpriteManager};

use slotmap::KeyData;
use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
};
use vulkano::{
    buffer::{Buffer, BufferContents, BufferCreateInfo, BufferUsage, Subbuffer},
    command_buffer::{
        AutoCommandBufferBuilder, CommandBufferInheritanceInfo, CommandBufferUsage,
        SecondaryAutoCommandBuffer, allocator::StandardCommandBufferAllocator,
    },
    descriptor_set::{
        PersistentDescriptorSet, WriteDescriptorSet, allocator::StandardDescriptorSetAllocator,
    },
    device::Queue,
    image::sampler::{Filter, Sampler, SamplerAddressMode, SamplerCreateInfo, SamplerMipmapMode},
    memory::allocator::{AllocationCreateInfo, MemoryTypeFilter, StandardMemoryAllocator},
    pipeline::{
        DynamicState, GraphicsPipeline, Pipeline, PipelineBindPoint, PipelineLayout,
        PipelineShaderStageCreateInfo,
        graphics::{
            GraphicsPipelineCreateInfo,
            color_blend::{ColorBlendAttachmentState, ColorBlendState},
            input_assembly::InputAssemblyState,
            multisample::MultisampleState,
            rasterization::RasterizationState,
            vertex_input::{Vertex, VertexDefinition},
            viewport::{Viewport, ViewportState},
        },
        layout::PipelineDescriptorSetLayoutCreateInfo,
    },
    render_pass::Subpass,
};
use zurie_types::Object;
use zurie_types::SpriteHandle;

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
    sprite_manager: Arc<RwLock<SpriteManager>>,
    indices: Subbuffer<[u32]>,
}

impl ObjectDrawPipeline {
    pub fn new(
        app: &Renderer,
        subpass: Subpass,
        sprite_manager: Arc<RwLock<SpriteManager>>,
    ) -> anyhow::Result<Self> {
        let memory_allocator = Arc::new(StandardMemoryAllocator::new_default(app.device.clone()));
        let command_buffer_allocator = app.command_buffer_allocator.clone();

        let vertices = Buffer::from_iter(
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
            [
                // Counter-clockwise vertices for a proper square
                TriangleVertex {
                    vert_position: [-0.5, -0.5], // Bottom-left
                },
                TriangleVertex {
                    vert_position: [-0.5, 0.5], // Top-left
                },
                TriangleVertex {
                    vert_position: [0.5, 0.5], // Top-right
                },
                TriangleVertex {
                    vert_position: [0.5, -0.5], // Bottom-right
                },
            ],
        )
        .unwrap();
        let indices = Buffer::from_iter(
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
            [0u32, 1, 2, 0, 2, 3], // Counter-clockwise triangle indices
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
                .definition(&vs.info().input_interface)?;
            let stages = [
                PipelineShaderStageCreateInfo::new(vs),
                PipelineShaderStageCreateInfo::new(fs),
            ];
            let layout = PipelineLayout::new(
                app.device.clone(),
                PipelineDescriptorSetLayoutCreateInfo::from_stages(&stages)
                    .into_pipeline_layout_create_info(app.device.clone())?,
            )?;

            GraphicsPipeline::new(
                app.device.clone(),
                None,
                GraphicsPipelineCreateInfo {
                    stages: stages.into_iter().collect(),
                    vertex_input_state: Some(vertex_input_state),
                    input_assembly_state: Some(InputAssemblyState::default()),
                    viewport_state: Some(ViewportState::default()),
                    rasterization_state: Some(RasterizationState::default()),
                    multisample_state: Some(MultisampleState::default()),
                    // Modify the color blend state for transparency
                    color_blend_state: Some(ColorBlendState::with_attachment_states(
                        subpass.num_color_attachments(),
                        ColorBlendAttachmentState {
                            blend: Some(vulkano::pipeline::graphics::color_blend::AttachmentBlend {
                                color_blend_op: vulkano::pipeline::graphics::color_blend::BlendOp::Add,
                                src_color_blend_factor: vulkano::pipeline::graphics::color_blend::BlendFactor::SrcAlpha,
                                dst_color_blend_factor: vulkano::pipeline::graphics::color_blend::BlendFactor::OneMinusSrcAlpha,
                                alpha_blend_op: vulkano::pipeline::graphics::color_blend::BlendOp::Add,
                                src_alpha_blend_factor: vulkano::pipeline::graphics::color_blend::BlendFactor::One,
                                dst_alpha_blend_factor: vulkano::pipeline::graphics::color_blend::BlendFactor::OneMinusSrcAlpha,
                            }),
                            ..Default::default()
                        },
                    )),
                    dynamic_state: [DynamicState::Viewport].into_iter().collect(),
                    subpass: Some(subpass.clone().into()),
                    ..GraphicsPipelineCreateInfo::layout(layout)
                },
            )?
        };

        let gfx_queue = app.gfx_queue();

        Ok(Self {
            gfx_queue,
            subpass,
            pipeline,
            command_buffer_allocator,
            descriptor_set_allocator: app.descriptor_set_allocator.clone(),
            memory_allocator,
            vertices,

            sprite_manager,
            indices,
        })
    }

    fn create_descriptor(
        &self,
        camera: vs::Camera,
        sprite: SpriteHandle,
    ) -> Arc<PersistentDescriptorSet> {
        let layout = self
            .pipeline
            .layout()
            .set_layouts()
            .first()
            .expect("No set layout found");
        let sampler = Sampler::new(self.gfx_queue.device().clone(), SamplerCreateInfo {
            mag_filter: Filter::Nearest,
            min_filter: Filter::Nearest,
            address_mode: [SamplerAddressMode::Repeat; 3],
            mipmap_mode: SamplerMipmapMode::Nearest,
            ..Default::default()
        })
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
        .expect("Failed to create camera buffer");

        PersistentDescriptorSet::new(
            &self.descriptor_set_allocator,
            layout.clone(),
            [
                WriteDescriptorSet::buffer(0, camera_buffer),
                WriteDescriptorSet::sampler(1, sampler),
                WriteDescriptorSet::image_view(
                    2,
                    self.sprite_manager
                        .write()
                        .expect("Failed to acquire sprite manager lock")
                        .get_texture(sprite)
                        .expect("Failed to get sprite texture"),
                ),
            ],
            [],
        )
        .expect("Failed to create descriptor set")
    }

    /// Draws input `image` over a quad of size -1.0 to 1.0.
    pub fn draw(
        &self,
        viewport_dimensions: [u32; 2],
        camera: vs::Camera,
        objects: Arc<RwLock<Vec<Object>>>,
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
        let mut objects_by_texture: HashMap<SpriteHandle, Vec<InstanceData>> = Default::default();
        for obj in objects.read().unwrap().iter() {
            objects_by_texture
                .entry(KeyData::from_ffi(obj.sprite).into())
                .or_default()
                .push(InstanceData {
                    position: obj.position.into(),
                    scale: obj.scale,
                    color: obj.color,
                });
        }
        for (sprite, objects) in objects_by_texture {
            let desc_set = self.create_descriptor(camera, sprite);
            let instance_data: Vec<InstanceData> = objects
                .iter()
                .map(|obj| InstanceData {
                    position: obj.position,
                    scale: obj.scale,
                    color: obj.color,
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
                .bind_index_buffer(self.indices.clone())
                .unwrap()
                .draw_indexed(6, instance_buffer_len as u32, 0, 0, 0)
                .unwrap();
        }
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
