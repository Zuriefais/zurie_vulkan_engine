use crate::render::Renderer;
use glam::IVec2;
use log::info;
use png::{BitDepth, ColorType, Encoder};
use std::sync::Arc;
use vulkano::{
    buffer::{Buffer, BufferCreateInfo, BufferUsage, Subbuffer},
    command_buffer::{
        allocator::StandardCommandBufferAllocator, AutoCommandBufferBuilder, CommandBufferUsage,
        PrimaryAutoCommandBuffer,
    },
    descriptor_set::{
        allocator::StandardDescriptorSetAllocator, PersistentDescriptorSet, WriteDescriptorSet,
    },
    device::Queue,
    format::Format,
    image::{view::ImageView, Image, ImageCreateInfo, ImageType, ImageUsage},
    memory::allocator::{AllocationCreateInfo, MemoryTypeFilter, StandardMemoryAllocator},
    pipeline::{
        compute::ComputePipelineCreateInfo, layout::PipelineDescriptorSetLayoutCreateInfo,
        ComputePipeline, Pipeline, PipelineBindPoint, PipelineLayout,
        PipelineShaderStageCreateInfo,
    },
    sync::GpuFuture,
};

pub struct RenderComputePipeline {
    compute_queue: Arc<Queue>,
    compute_grid_pipeline: Arc<ComputePipeline>,
    command_buffer_allocator: Arc<StandardCommandBufferAllocator>,
    descriptor_set_allocator: Arc<StandardDescriptorSetAllocator>,
    pub grid_in: Subbuffer<[u32]>,
    pub grid_out: Subbuffer<[u32]>,
    image: Arc<ImageView>,
    memory_allocator: Arc<
        vulkano::memory::allocator::GenericMemoryAllocator<
            vulkano::memory::allocator::FreeListAllocator,
        >,
    >,
    size: [u32; 2],
}

fn rand_grid(memory_allocator: Arc<StandardMemoryAllocator>, size: [u32; 2]) -> Subbuffer<[u32]> {
    Buffer::from_iter(
        memory_allocator,
        BufferCreateInfo {
            usage: BufferUsage::STORAGE_BUFFER,
            ..Default::default()
        },
        AllocationCreateInfo {
            memory_type_filter: MemoryTypeFilter::PREFER_DEVICE
                | MemoryTypeFilter::HOST_SEQUENTIAL_WRITE,
            ..Default::default()
        },
        (0..(size[0] * size[1])).map(|_| fastrand::bool() as u32),
    )
    .unwrap()
}

impl RenderComputePipeline {
    pub fn new(app: &Renderer) -> RenderComputePipeline {
        let compute_queue = app.compute_queue().clone();
        let size = app.window_size();
        let memory_allocator = app.memory_allocator.clone();
        let grid_in = rand_grid(memory_allocator.clone(), size);
        let grid_out = rand_grid(memory_allocator.clone(), size);

        let compute_grid_pipeline = {
            let device = app.compute_queue.device();
            let cs = compute_grid_cs::load(device.clone())
                .unwrap()
                .entry_point("main")
                .unwrap();
            let stage = PipelineShaderStageCreateInfo::new(cs);
            let layout = PipelineLayout::new(
                device.clone(),
                PipelineDescriptorSetLayoutCreateInfo::from_stages([&stage])
                    .into_pipeline_layout_create_info(device.clone())
                    .unwrap(),
            )
            .unwrap();

            ComputePipeline::new(
                device.clone(),
                None,
                ComputePipelineCreateInfo::stage_layout(stage, layout),
            )
            .unwrap()
        };

        let image = RenderComputePipeline::new_image(memory_allocator.clone(), size);

        RenderComputePipeline {
            compute_queue,
            compute_grid_pipeline,
            command_buffer_allocator: app.command_buffer_allocator.clone(),
            descriptor_set_allocator: app.descriptor_set_allocator.clone(),
            grid_in,
            grid_out,
            image,
            memory_allocator,
            size,
        }
    }

    fn new_image(memory_allocator: Arc<StandardMemoryAllocator>, size: [u32; 2]) -> Arc<ImageView> {
        ImageView::new_default(
            Image::new(
                memory_allocator.clone(),
                ImageCreateInfo {
                    image_type: ImageType::Dim2d,
                    format: Format::R8G8B8A8_UNORM,
                    extent: [size[0], size[1], 1],
                    usage: ImageUsage::TRANSFER_DST | ImageUsage::SAMPLED | ImageUsage::STORAGE,
                    ..Default::default()
                },
                AllocationCreateInfo::default(),
            )
            .unwrap(),
        )
        .unwrap()
    }

    pub fn color_image(&self) -> Arc<ImageView> {
        self.image.clone()
    }

    pub fn draw_grid(&self, pos: IVec2) {
        let pos = pos / 4;
        let mut grid_in = self.grid_in.write().unwrap();
        let extent = self.image.image().extent();
        if pos.y < 0 || pos.y >= extent[1] as i32 || pos.x < 0 || pos.x >= extent[0] as i32 {
            return;
        }
        info!("drawing on grid");
        let index = (pos.y * extent[0] as i32 + pos.x) as usize; // Use unscaled pos\
        info!("trying to draw on grid: {}, {}", pos, index);
        grid_in[index] = 1;
    }

    pub fn compute(
        &mut self,
        before_future: Box<dyn GpuFuture>,
        simulate: &bool,
    ) -> Box<dyn GpuFuture> {
        if !simulate {
            return before_future;
        }
        let mut builder = AutoCommandBufferBuilder::primary(
            self.command_buffer_allocator.as_ref(),
            self.compute_queue.queue_family_index(),
            CommandBufferUsage::OneTimeSubmit,
        )
        .unwrap();
        let sand_color = [0.149, 0.169, 0.094, 1.0];

        self.dispatch(&mut builder, sand_color);

        let command_buffer = builder.build().unwrap();
        let finished = before_future
            .then_execute(self.compute_queue.clone(), command_buffer)
            .unwrap();
        let after_pipeline = finished.then_signal_fence_and_flush().unwrap().boxed();

        std::mem::swap(&mut self.grid_in, &mut self.grid_out);

        after_pipeline
    }

    fn dispatch(
        &self,
        builder: &mut AutoCommandBufferBuilder<PrimaryAutoCommandBuffer>,
        sand_color: [f32; 4],
    ) {
        let image_extent = self.image.image().extent();
        let pipeline_layout = self.compute_grid_pipeline.layout();
        let desc_layout = pipeline_layout.set_layouts().first().unwrap();
        let set = PersistentDescriptorSet::new(
            &self.descriptor_set_allocator,
            desc_layout.clone(),
            [
                WriteDescriptorSet::image_view(0, self.image.clone()),
                WriteDescriptorSet::buffer(1, self.grid_in.clone()),
                WriteDescriptorSet::buffer(2, self.grid_out.clone()),
            ],
            [],
        )
        .unwrap();

        let push_constants = compute_grid_cs::PushConstants { sand_color };
        builder
            .bind_pipeline_compute(self.compute_grid_pipeline.clone())
            .unwrap()
            .bind_descriptor_sets(PipelineBindPoint::Compute, pipeline_layout.clone(), 0, set)
            .unwrap()
            .push_constants(pipeline_layout.clone(), 0, push_constants)
            .unwrap()
            .dispatch([image_extent[0] / 8, image_extent[1] / 8, 1])
            .unwrap();
    }

    pub fn resize(&mut self, size: [u32; 2]) {
        let size = [size[0] / SCALE_FACTOR, size[1] / SCALE_FACTOR];
        self.image = RenderComputePipeline::new_image(self.memory_allocator.clone(), size);
        self.grid_in = rand_grid(self.memory_allocator.clone(), size);
        self.grid_out = rand_grid(self.memory_allocator.clone(), size);
        self.size = size;
    }

    pub fn new_rand_grid(&mut self) {
        self.grid_in = rand_grid(self.memory_allocator.clone(), self.size);
        self.grid_out = rand_grid(self.memory_allocator.clone(), self.size);
    }
}

mod compute_grid_cs {
    vulkano_shaders::shader! {
        ty: "compute",
        path: "shaders/compute/render.glsl"
    }
}

const SCALE_FACTOR: u32 = 4;
