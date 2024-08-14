use crate::render::Renderer;
use glam::{IVec2, Vec2};
use log::info;
use std::f64::consts::PI;
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

pub struct SandComputePipeline {
    compute_queue: Arc<Queue>,
    compute_grid_pipeline: Arc<ComputePipeline>,
    command_buffer_allocator: Arc<StandardCommandBufferAllocator>,
    descriptor_set_allocator: Arc<StandardDescriptorSetAllocator>,
    pub grid: Subbuffer<[u32]>,
    image: Arc<ImageView>,
    memory_allocator: Arc<
        vulkano::memory::allocator::GenericMemoryAllocator<
            vulkano::memory::allocator::FreeListAllocator,
        >,
    >,
    size: [u32; 2],
}

fn get_pos(index: usize, dims: [u32; 2]) -> Option<IVec2> {
    if index >= (dims[0] * dims[1]) as usize {
        return None; // Handle out-of-bounds index
    }

    let y = index / dims[0] as usize;
    let x = index % dims[0] as usize;
    Some(IVec2::new(x as i32, y as i32))
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
        (0..(size[0] * size[1])).map(|i| {
            if let Some(value) = get_pos(i as usize, size) {
                if value.y == 0
                    || value.y == size[1] as i32 - 1
                    || value.x == 0
                    || value.x == size[0] as i32 - 2
                {
                    CellType::Wall as u32
                } else {
                    fastrand::u32(0..2)
                }
            } else {
                fastrand::u32(0..2)
            }
        }),
    )
    .unwrap()
}

impl SandComputePipeline {
    pub fn new(app: &Renderer) -> SandComputePipeline {
        let compute_queue = app.compute_queue().clone();
        let size = app.window_size();
        let memory_allocator = app.memory_allocator.clone();
        let grid = rand_grid(memory_allocator.clone(), size);

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

        let image = SandComputePipeline::new_image(memory_allocator.clone(), size);

        SandComputePipeline {
            compute_queue,
            compute_grid_pipeline,
            command_buffer_allocator: app.command_buffer_allocator.clone(),
            descriptor_set_allocator: app.descriptor_set_allocator.clone(),
            grid,
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
        let mut grid_in = self.grid.write().unwrap();
        let extent = self.image.image().extent();
        if pos.y < 0 || pos.y >= extent[1] as i32 || pos.x < 0 || pos.x >= extent[0] as i32 {
            return;
        }
        info!("drawing on grid");
        let index = (pos.y * extent[0] as i32 + pos.x) as usize; // Use unscaled pos\
        info!("trying to draw on grid: {}, {}", pos, index);
        grid_in[index] = 1;
    }

    pub fn draw_circle(&self, pos: Vec2, r: i32, window_size: [u32; 2]) {
        let mut normalized_pos = Vec2::new(
            (pos.x / window_size[0] as f32).clamp(0.0, 1.0),
            (pos.y / window_size[1] as f32).clamp(0.0, 1.0),
        );

        normalized_pos.y = 1.0 - normalized_pos.y;
        let pos = IVec2::new(
            (self.size[0] as f32 * normalized_pos.x) as i32,
            (self.size[1] as f32 * normalized_pos.y) as i32,
        );
        let mut grid_in = self.grid.write().unwrap();
        let extent = self.image.image().extent();
        for i in (0..3600).map(|i| i as f64 / 10.0) {
            let angle = i;
            let x = (r as f64 * (angle * PI / 180.0).cos()).round() as i32;
            let y = (r as f64 * (angle * PI / 180.0).sin()).round() as i32;

            let add_pos = IVec2::new(x, y);
            let pos = pos + add_pos;
            let index = (pos.y * extent[0] as i32 + pos.x) as usize;
            grid_in[index] = 1;
        }
    }

    pub fn compute(
        &mut self,
        before_future: Box<dyn GpuFuture>,
        simulate: bool,
    ) -> Box<dyn GpuFuture> {
        let mut builder = AutoCommandBufferBuilder::primary(
            self.command_buffer_allocator.as_ref(),
            self.compute_queue.queue_family_index(),
            CommandBufferUsage::OneTimeSubmit,
        )
        .unwrap();
        let sand_color = [0.149, 0.169, 0.094, 1.0];
        self.dispatch(
            &mut builder,
            [
                [0.0; 4],
                sand_color,
                [0.302, 0.267, 0.255, 1.0],
                [0.431, 0.318, 0.251, 1.0],
            ],
            simulate,
        );

        let command_buffer = builder.build().unwrap();
        let finished = before_future
            .then_execute(self.compute_queue.clone(), command_buffer)
            .unwrap();

        (finished.then_signal_fence_and_flush().unwrap().boxed()) as _
    }

    fn dispatch(
        &self,
        builder: &mut AutoCommandBufferBuilder<PrimaryAutoCommandBuffer>,
        palette: [[f32; 4]; 4],
        simulate: bool,
    ) {
        let image_extent = self.image.image().extent();
        let pipeline_layout = self.compute_grid_pipeline.layout();
        let desc_layout = pipeline_layout.set_layouts().first().unwrap();
        let set = PersistentDescriptorSet::new(
            &self.descriptor_set_allocator,
            desc_layout.clone(),
            [
                WriteDescriptorSet::image_view(0, self.image.clone()),
                WriteDescriptorSet::buffer(1, self.grid.clone()),
            ],
            [],
        )
        .unwrap();
        let simulate = if simulate { 1 } else { 0 };
        let push_constants = compute_grid_cs::PushConstants { palette, simulate };
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
        self.image = SandComputePipeline::new_image(self.memory_allocator.clone(), size);
        self.size = size;
        self.new_rand_grid();
    }

    pub fn new_rand_grid(&mut self) {
        info!("generating new rand grid.... Size: {:?}", self.size);
        self.grid = rand_grid(self.memory_allocator.clone(), self.size);
    }
}

mod compute_grid_cs {
    vulkano_shaders::shader! {
        ty: "compute",
        path: "shaders/compute/sand.glsl"
    }
}

const SCALE_FACTOR: u32 = 4;

pub enum CellType {
    Empty,
    Sand,
    Wall,
    Water,
}
