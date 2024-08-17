#![feature(prelude_import)]
#[prelude_import]
use std::prelude::rust_2021::*;
#[macro_use]
extern crate std;
pub mod app {
    use crate::state::State;
    use log::info;
    use std::sync::Arc;
    use winit::{
        application::ApplicationHandler,
        event::{ElementState, KeyEvent, WindowEvent},
        event_loop::ActiveEventLoop,
        keyboard::{KeyCode, PhysicalKey},
        window::Window,
    };
    pub struct App {
        window: Option<Arc<Window>>,
        state: Option<State>,
    }
    #[automatically_derived]
    impl ::core::default::Default for App {
        #[inline]
        fn default() -> App {
            App {
                window: ::core::default::Default::default(),
                state: ::core::default::Default::default(),
            }
        }
    }
    impl ApplicationHandler for App {
        fn resumed(&mut self, event_loop: &ActiveEventLoop) {
            {
                let lvl = ::log::Level::Info;
                if lvl <= ::log::STATIC_MAX_LEVEL && lvl <= ::log::max_level() {
                    ::log::__private_api::log(
                        format_args!("Creating window"),
                        lvl,
                        &(
                            "vulkan_engine::app",
                            "vulkan_engine::app",
                            ::log::__private_api::loc(),
                        ),
                        (),
                    );
                }
            };
            if self.window.is_none() {
                let window_attributes =
                    Window::default_attributes().with_title("Vulcan engine by Zuriefais");
                let window = Arc::new(event_loop.create_window(window_attributes).unwrap());
                self.window = Some(window.clone());
                let state = pollster::block_on(State::new(window.clone(), event_loop));
                self.state = Some(state);
            }
        }
        fn window_event(
            &mut self,
            event_loop: &winit::event_loop::ActiveEventLoop,
            _: winit::window::WindowId,
            event: winit::event::WindowEvent,
        ) {
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
                WindowEvent::Resized(size) => self
                    .state
                    .as_mut()
                    .unwrap()
                    .resize([size.width, size.height]),
                WindowEvent::RedrawRequested => {
                    self.state.as_mut().unwrap().render();
                    self.window.as_ref().unwrap().request_redraw();
                }
                event => self.state.as_mut().unwrap().event(event),
            }
        }
    }
}
pub mod compute_sand {
    use crate::render::Renderer;
    use egui_winit_vulkano::egui::ImeEvent;
    use glam::{IVec2, Vec2};
    use log::info;
    use std::f64::consts::PI;
    use std::sync::Arc;
    use strum_macros::{Display, EnumIter};
    use vulkano::{
        buffer::{Buffer, BufferCreateInfo, BufferUsage, BufferWriteGuard, Subbuffer},
        command_buffer::{
            allocator::StandardCommandBufferAllocator, AutoCommandBufferBuilder,
            CommandBufferUsage, PrimaryAutoCommandBuffer,
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
        pub scale_factor: u32,
        pub pallete: [[f32; 4]; 4],
        pub brush_size: u32,
        pub selected_brush: BrushType,
    }
    fn get_pos(index: usize, dims: [u32; 2]) -> Option<IVec2> {
        if index >= (dims[0] * dims[1]) as usize {
            return None;
        }
        let y = index / dims[0] as usize;
        let x = index % dims[0] as usize;
        Some(IVec2::new(x as i32, y as i32))
    }
    fn rand_grid(
        memory_allocator: Arc<StandardMemoryAllocator>,
        size: [u32; 2],
    ) -> Subbuffer<[u32]> {
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
                scale_factor: 4,
                pallete: [
                    [0.0; 4],
                    [0.149, 0.169, 0.094, 1.0],
                    [0.302, 0.267, 0.255, 1.0],
                    [0.431, 0.318, 0.251, 1.0],
                ],
                brush_size: 5,
                selected_brush: BrushType::CircleFull,
            }
        }
        fn new_image(
            memory_allocator: Arc<StandardMemoryAllocator>,
            size: [u32; 2],
        ) -> Arc<ImageView> {
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
        pub fn draw(&self, pos: Vec2, window_size: [u32; 2], material: CellType) {
            let normalized_pos = self.normalize_mouse_pos(pos, window_size);
            match self.selected_brush {
                BrushType::CircleFull => self.draw_circle(normalized_pos, material),
                BrushType::CircleHollow => {
                    let mut grid_in = self.grid.write().unwrap();
                    let extent = self.image.image().extent();
                    draw_circle_hollow(
                        self.brush_size as f64,
                        normalized_pos,
                        extent,
                        &mut grid_in,
                        material,
                    )
                }
                BrushType::Cube => {
                    let extent = self.image.image().extent();
                    self.draw_cube(normalized_pos, material, extent)
                }
            }
        }
        pub fn draw_grid(&self, pos: IVec2) {
            let pos = pos / 4;
            let mut grid_in = self.grid.write().unwrap();
            let extent = self.image.image().extent();
            if pos.y < 0 || pos.y >= extent[1] as i32 || pos.x < 0 || pos.x >= extent[0] as i32 {
                return;
            }
            {
                let lvl = ::log::Level::Info;
                if lvl <= ::log::STATIC_MAX_LEVEL && lvl <= ::log::max_level() {
                    ::log::__private_api::log(
                        format_args!("drawing on grid"),
                        lvl,
                        &(
                            "vulkan_engine::compute_sand",
                            "vulkan_engine::compute_sand",
                            ::log::__private_api::loc(),
                        ),
                        (),
                    );
                }
            };
            let index = (pos.y * extent[0] as i32 + pos.x) as usize;
            {
                let lvl = ::log::Level::Info;
                if lvl <= ::log::STATIC_MAX_LEVEL && lvl <= ::log::max_level() {
                    ::log::__private_api::log(
                        format_args!("trying to draw on grid: {0}, {1}", pos, index),
                        lvl,
                        &(
                            "vulkan_engine::compute_sand",
                            "vulkan_engine::compute_sand",
                            ::log::__private_api::loc(),
                        ),
                        (),
                    );
                }
            };
            grid_in[index] = 1;
        }
        pub fn draw_circle(&self, pos: IVec2, material: CellType) {
            let mut grid_in = self.grid.write().unwrap();
            let extent = self.image.image().extent();
            for radius in 0..=self.brush_size as i32 {
                draw_circle_hollow(radius as f64, pos, extent, &mut grid_in, material);
            }
        }
        pub fn draw_cube(&self, pos: IVec2, material: CellType, extent: [u32; 3]) {
            let mut grid_in = self.grid.write().unwrap();
            for x in (pos.x - (self.brush_size as i32 - 1) / 2)
                ..(pos.x + (self.brush_size as i32 - 1) / 2)
            {
                for y in (pos.y - (self.brush_size as i32 - 1) / 2)
                    ..(pos.y + (self.brush_size as i32 - 1) / 2)
                {
                    let pos = IVec2::new(x, y);
                    let index = (pos.y * extent[0] as i32 + pos.x) as usize;
                    draw_pixel(&mut grid_in, index, material)
                }
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
            self.dispatch(&mut builder, self.pallete, simulate);
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
            let size = [size[0] / self.scale_factor, size[1] / self.scale_factor];
            self.image = SandComputePipeline::new_image(self.memory_allocator.clone(), size);
            self.size = size;
            self.new_rand_grid();
        }
        pub fn new_rand_grid(&mut self) {
            {
                let lvl = ::log::Level::Info;
                if lvl <= ::log::STATIC_MAX_LEVEL && lvl <= ::log::max_level() {
                    ::log::__private_api::log(
                        format_args!("generating new rand grid.... Size: {0:?}", self.size,),
                        lvl,
                        &(
                            "vulkan_engine::compute_sand",
                            "vulkan_engine::compute_sand",
                            ::log::__private_api::loc(),
                        ),
                        (),
                    );
                }
            };
            self.grid = rand_grid(self.memory_allocator.clone(), self.size);
        }
        pub fn normalize_mouse_pos(&self, pos: Vec2, window_size: [u32; 2]) -> IVec2 {
            let mut normalized_pos = Vec2::new(
                (pos.x / window_size[0] as f32).clamp(0.0, 1.0),
                (pos.y / window_size[1] as f32).clamp(0.0, 1.0),
            );
            normalized_pos.y = 1.0 - normalized_pos.y;
            IVec2::new(
                (self.size[0] as f32 * normalized_pos.x) as i32,
                (self.size[1] as f32 * normalized_pos.y) as i32,
            )
        }
    }
    fn draw_circle_hollow(
        radius: f64,
        pos: IVec2,
        extent: [u32; 3],
        grid_in: &mut BufferWriteGuard<[u32]>,
        material: CellType,
    ) {
        for i in (0..3600).map(|i| i as f64 / 10.0) {
            let angle = i;
            let x = (radius * (angle * PI / 180.0).cos()).round() as i32;
            let y = (radius * (angle * PI / 180.0).sin()).round() as i32;
            let add_pos = IVec2::new(x, y);
            let pos = pos + add_pos;
            let index = (pos.y * extent[0] as i32 + pos.x) as usize;
            draw_pixel(grid_in, index, material)
        }
    }
    pub fn draw_pixel(grid: &mut BufferWriteGuard<[u32]>, i: usize, cell_type: CellType) {
        if i < grid.len() {
            grid[i] = cell_type as u32;
        }
    }
    mod compute_grid_cs {
        /// Loads the shader as a `ShaderModule`.
        #[allow(unsafe_code)]
        #[inline]
        pub fn load(
            device: ::std::sync::Arc<::vulkano::device::Device>,
        ) -> ::std::result::Result<
            ::std::sync::Arc<::vulkano::shader::ShaderModule>,
            ::vulkano::Validated<::vulkano::VulkanError>,
        > {
            let _bytes = (b"#version 450\n\nlayout(local_size_x = 32, local_size_y = 32, local_size_z = 1) in;\n\nlayout(set = 0, binding = 0, rgba8) uniform writeonly image2D img;\nlayout(set = 0, binding = 1) buffer GridBuffer {\n    uint grid[];\n};\n\nlayout(push_constant) uniform PushConstants {\n    vec4[4] palette;\n    bool simulate;\n} push_constants;\n\nint get_index(ivec2 pos) {\n    ivec2 dims = ivec2(imageSize(img));\n    return pos.y * dims.x + pos.x;\n}\n\n#define EMPTY 0\n#define SAND 1\n#define WALL 2\n#define WATER 3\n\nvoid sand(ivec2 pixelCoord, ivec2 imgSize) {\n    ivec2 below = pixelCoord + ivec2(0, -1);\n\n    if (below.y >= imgSize.y || atomicExchange(grid[below.y * imgSize.x + below.x], SAND) == EMPTY) {\n        atomicExchange(grid[pixelCoord.y * imgSize.x + pixelCoord.x], EMPTY);\n    } else if (atomicExchange(grid[below.y * imgSize.x + below.x], SAND) == WATER) {\n        atomicExchange(grid[pixelCoord.y * imgSize.x + pixelCoord.x], WATER);\n    } else {\n        ivec2 belowLeft = pixelCoord + ivec2(-1, -1);\n        ivec2 belowRight = pixelCoord + ivec2(1, -1);\n\n        bool canFallLeft = (belowLeft.x >= 0 && belowLeft.y < imgSize.y &&\n                (atomicExchange(grid[belowLeft.y * imgSize.x + belowLeft.x], SAND) == EMPTY));\n        bool canFallRight = (belowRight.x < imgSize.x && belowRight.y < imgSize.y &&\n                (atomicExchange(grid[belowRight.y * imgSize.x + belowRight.x], SAND) == EMPTY));\n\n        if (canFallLeft && canFallRight) {\n            if (gl_GlobalInvocationID.x % 2 == 0) {\n                atomicExchange(grid[pixelCoord.y * imgSize.x + pixelCoord.x], EMPTY);\n            } else {\n                atomicExchange(grid[pixelCoord.y * imgSize.x + pixelCoord.x], EMPTY);\n            }\n        } else if (canFallLeft) {\n            atomicExchange(grid[pixelCoord.y * imgSize.x + pixelCoord.x], EMPTY);\n        } else if (canFallRight) {\n            atomicExchange(grid[pixelCoord.y * imgSize.x + pixelCoord.x], EMPTY);\n        } else {\n            // Sand stays in place\n        }\n    }\n}\n\nvoid water(ivec2 pixelCoord, ivec2 imgSize) {\n    ivec2 below = pixelCoord + ivec2(0, -1);\n\n    if (below.y >= imgSize.y || atomicExchange(grid[below.y * imgSize.x + below.x], WATER) == EMPTY) {\n        atomicExchange(grid[pixelCoord.y * imgSize.x + pixelCoord.x], EMPTY);\n    } else {\n        ivec2 belowLeft = pixelCoord + ivec2(-1, -1);\n        ivec2 belowRight = pixelCoord + ivec2(1, -1);\n\n        bool canFallLeft = (belowLeft.x >= 0 && belowLeft.y < imgSize.y &&\n                (atomicExchange(grid[belowLeft.y * imgSize.x + belowLeft.x], WATER) == EMPTY));\n        bool canFallRight = (belowRight.x < imgSize.x && belowRight.y < imgSize.y &&\n                (atomicExchange(grid[belowRight.y * imgSize.x + belowRight.x], WATER) == EMPTY));\n\n        if (canFallLeft && canFallRight) {\n            if (gl_GlobalInvocationID.x % 2 == 0) {\n                atomicExchange(grid[pixelCoord.y * imgSize.x + pixelCoord.x], EMPTY);\n            } else {\n                atomicExchange(grid[pixelCoord.y * imgSize.x + pixelCoord.x], EMPTY);\n            }\n        } else if (canFallLeft) {\n            atomicExchange(grid[pixelCoord.y * imgSize.x + pixelCoord.x], EMPTY);\n        } else if (canFallRight) {\n            atomicExchange(grid[pixelCoord.y * imgSize.x + pixelCoord.x], EMPTY);\n        } else {\n            // Water stays in place\n        }\n        ivec2 left = pixelCoord + ivec2(-1, 0);\n        ivec2 right = pixelCoord + ivec2(1, 0);\n\n        bool canSlideLeft = (belowLeft.x >= 0 && belowLeft.y < imgSize.y &&\n                atomicExchange(grid[left.y * imgSize.x + left.x], WATER) == EMPTY);\n        bool canSlideRight = (belowRight.x < imgSize.x && belowRight.y < imgSize.y &&\n                atomicExchange(grid[right.y * imgSize.x + right.x], WATER) == EMPTY);\n\n        if (canSlideLeft && canSlideRight) {\n            if (gl_GlobalInvocationID.x % 2 == 0) {\n                atomicExchange(grid[pixelCoord.y * imgSize.x + pixelCoord.x], EMPTY);\n            } else {\n                atomicExchange(grid[pixelCoord.y * imgSize.x + pixelCoord.x], EMPTY);\n            }\n        } else if (canSlideLeft) {\n            atomicExchange(grid[pixelCoord.y * imgSize.x + pixelCoord.x], EMPTY);\n        } else if (canSlideRight) {\n            atomicExchange(grid[pixelCoord.y * imgSize.x + pixelCoord.x], EMPTY);\n        }\n    }\n}\n\nvoid simulate(ivec2 pixelCoord, ivec2 imgSize) {\n    if (pixelCoord.x >= imgSize.x || pixelCoord.y >= imgSize.y) {\n        return;\n    }\n\n    uint cellValue = grid[pixelCoord.y * imgSize.x + pixelCoord.x];\n\n    if (cellValue == SAND) {\n        sand(pixelCoord, imgSize);\n    } else if (cellValue == WATER) {\n        water(pixelCoord, imgSize);\n    } else if (cellValue == WALL) {\n        // Wall stays in place\n    }\n    else {\n        // Empty cell stays empty\n    }\n}\n\nvoid main() {\n    ivec2 imgSize = imageSize(img);\n    ivec2 pixelCoord = ivec2(gl_GlobalInvocationID.xy);\n\n    if (push_constants.simulate) {\n        simulate(pixelCoord, imgSize);\n    }\n    barrier();\n\n    vec4 color = push_constants.palette[grid[pixelCoord.y * imgSize.x + pixelCoord.x]];\n    imageStore(img, pixelCoord, color);\n}\n");
            static WORDS: &[u32] = &[
                119734787u32,
                65536u32,
                851979u32,
                606u32,
                0u32,
                131089u32,
                1u32,
                131089u32,
                50u32,
                393227u32,
                1u32,
                1280527431u32,
                1685353262u32,
                808793134u32,
                0u32,
                196622u32,
                0u32,
                1u32,
                393231u32,
                5u32,
                4u32,
                1852399981u32,
                0u32,
                167u32,
                393232u32,
                4u32,
                17u32,
                32u32,
                32u32,
                1u32,
                196611u32,
                2u32,
                450u32,
                655364u32,
                1197427783u32,
                1279741775u32,
                1885560645u32,
                1953718128u32,
                1600482425u32,
                1701734764u32,
                1919509599u32,
                1769235301u32,
                25974u32,
                524292u32,
                1197427783u32,
                1279741775u32,
                1852399429u32,
                1685417059u32,
                1768185701u32,
                1952671090u32,
                6649449u32,
                262149u32,
                4u32,
                1852399981u32,
                0u32,
                393221u32,
                12u32,
                1684955507u32,
                845772328u32,
                845772347u32,
                59u32,
                327685u32,
                10u32,
                1702390128u32,
                1869562732u32,
                25714u32,
                262149u32,
                11u32,
                1399287145u32,
                6650473u32,
                393221u32,
                16u32,
                1702125943u32,
                1769351282u32,
                1769356082u32,
                15154u32,
                327685u32,
                14u32,
                1702390128u32,
                1869562732u32,
                25714u32,
                262149u32,
                15u32,
                1399287145u32,
                6650473u32,
                458757u32,
                20u32,
                1970104691u32,
                1702125932u32,
                845772328u32,
                845772347u32,
                59u32,
                327685u32,
                18u32,
                1702390128u32,
                1869562732u32,
                25714u32,
                262149u32,
                19u32,
                1399287145u32,
                6650473u32,
                262149u32,
                22u32,
                1869374818u32,
                119u32,
                327685u32,
                41u32,
                1684632135u32,
                1717990722u32,
                29285u32,
                327686u32,
                41u32,
                0u32,
                1684632167u32,
                0u32,
                196613u32,
                43u32,
                0u32,
                327685u32,
                96u32,
                1869374818u32,
                1717914743u32,
                116u32,
                327685u32,
                100u32,
                1869374818u32,
                1734955639u32,
                29800u32,
                327685u32,
                106u32,
                1181639011u32,
                1282174049u32,
                7628389u32,
                393221u32,
                132u32,
                1181639011u32,
                1382837345u32,
                1952999273u32,
                0u32,
                524293u32,
                167u32,
                1197436007u32,
                1633841004u32,
                1986939244u32,
                1952539503u32,
                1231974249u32,
                68u32,
                262149u32,
                225u32,
                1869374818u32,
                119u32,
                327685u32,
                261u32,
                1869374818u32,
                1717914743u32,
                116u32,
                327685u32,
                264u32,
                1869374818u32,
                1734955639u32,
                29800u32,
                327685u32,
                267u32,
                1181639011u32,
                1282174049u32,
                7628389u32,
                393221u32,
                293u32,
                1181639011u32,
                1382837345u32,
                1952999273u32,
                0u32,
                262149u32,
                381u32,
                1952867692u32,
                0u32,
                262149u32,
                385u32,
                1751607666u32,
                116u32,
                393221u32,
                389u32,
                1399742819u32,
                1701079404u32,
                1952867660u32,
                0u32,
                393221u32,
                415u32,
                1399742819u32,
                1701079404u32,
                1751607634u32,
                116u32,
                327685u32,
                521u32,
                1819043171u32,
                1970037078u32,
                101u32,
                262149u32,
                536u32,
                1634886000u32,
                109u32,
                262149u32,
                538u32,
                1634886000u32,
                109u32,
                262149u32,
                546u32,
                1634886000u32,
                109u32,
                262149u32,
                548u32,
                1634886000u32,
                109u32,
                262149u32,
                556u32,
                1399287145u32,
                6650473u32,
                196613u32,
                560u32,
                6778217u32,
                327685u32,
                563u32,
                1702390128u32,
                1869562732u32,
                25714u32,
                393221u32,
                571u32,
                1752397136u32,
                1936617283u32,
                1953390964u32,
                115u32,
                327686u32,
                571u32,
                0u32,
                1701601648u32,
                6648948u32,
                393222u32,
                571u32,
                1u32,
                1970104691u32,
                1702125932u32,
                0u32,
                393221u32,
                573u32,
                1752397168u32,
                1852793695u32,
                1851880563u32,
                29556u32,
                262149u32,
                580u32,
                1634886000u32,
                109u32,
                262149u32,
                582u32,
                1634886000u32,
                109u32,
                262149u32,
                587u32,
                1869377379u32,
                114u32,
                262215u32,
                40u32,
                6u32,
                4u32,
                327752u32,
                41u32,
                0u32,
                35u32,
                0u32,
                196679u32,
                41u32,
                3u32,
                262215u32,
                43u32,
                34u32,
                0u32,
                262215u32,
                43u32,
                33u32,
                1u32,
                262215u32,
                167u32,
                11u32,
                28u32,
                262215u32,
                560u32,
                34u32,
                0u32,
                262215u32,
                560u32,
                33u32,
                0u32,
                196679u32,
                560u32,
                25u32,
                262215u32,
                570u32,
                6u32,
                16u32,
                327752u32,
                571u32,
                0u32,
                35u32,
                0u32,
                327752u32,
                571u32,
                1u32,
                35u32,
                64u32,
                196679u32,
                571u32,
                2u32,
                262215u32,
                605u32,
                11u32,
                25u32,
                131091u32,
                2u32,
                196641u32,
                3u32,
                2u32,
                262165u32,
                6u32,
                32u32,
                1u32,
                262167u32,
                7u32,
                6u32,
                2u32,
                262176u32,
                8u32,
                7u32,
                7u32,
                327713u32,
                9u32,
                2u32,
                8u32,
                8u32,
                262187u32,
                6u32,
                24u32,
                0u32,
                262187u32,
                6u32,
                25u32,
                4294967295u32,
                327724u32,
                7u32,
                26u32,
                24u32,
                25u32,
                131092u32,
                28u32,
                262165u32,
                29u32,
                32u32,
                0u32,
                262187u32,
                29u32,
                30u32,
                1u32,
                262176u32,
                31u32,
                7u32,
                6u32,
                196637u32,
                40u32,
                29u32,
                196638u32,
                41u32,
                40u32,
                262176u32,
                42u32,
                2u32,
                41u32,
                262203u32,
                42u32,
                43u32,
                2u32,
                262187u32,
                29u32,
                46u32,
                0u32,
                262176u32,
                53u32,
                2u32,
                29u32,
                262187u32,
                29u32,
                81u32,
                3u32,
                327724u32,
                7u32,
                98u32,
                25u32,
                25u32,
                262187u32,
                6u32,
                102u32,
                1u32,
                327724u32,
                7u32,
                103u32,
                102u32,
                25u32,
                262176u32,
                105u32,
                7u32,
                28u32,
                262167u32,
                165u32,
                29u32,
                3u32,
                262176u32,
                166u32,
                1u32,
                165u32,
                262203u32,
                166u32,
                167u32,
                1u32,
                262176u32,
                168u32,
                1u32,
                29u32,
                262187u32,
                29u32,
                171u32,
                2u32,
                327724u32,
                7u32,
                383u32,
                25u32,
                24u32,
                327724u32,
                7u32,
                387u32,
                102u32,
                24u32,
                262176u32,
                520u32,
                7u32,
                29u32,
                196630u32,
                557u32,
                32u32,
                589849u32,
                558u32,
                557u32,
                1u32,
                0u32,
                0u32,
                0u32,
                2u32,
                4u32,
                262176u32,
                559u32,
                0u32,
                558u32,
                262203u32,
                559u32,
                560u32,
                0u32,
                262167u32,
                564u32,
                29u32,
                2u32,
                262167u32,
                568u32,
                557u32,
                4u32,
                262187u32,
                29u32,
                569u32,
                4u32,
                262172u32,
                570u32,
                568u32,
                569u32,
                262174u32,
                571u32,
                570u32,
                29u32,
                262176u32,
                572u32,
                9u32,
                571u32,
                262203u32,
                572u32,
                573u32,
                9u32,
                262176u32,
                574u32,
                9u32,
                29u32,
                262187u32,
                29u32,
                585u32,
                264u32,
                262176u32,
                586u32,
                7u32,
                568u32,
                262176u32,
                598u32,
                9u32,
                568u32,
                262187u32,
                29u32,
                604u32,
                32u32,
                393260u32,
                165u32,
                605u32,
                604u32,
                604u32,
                30u32,
                327734u32,
                2u32,
                4u32,
                0u32,
                3u32,
                131320u32,
                5u32,
                262203u32,
                8u32,
                556u32,
                7u32,
                262203u32,
                8u32,
                563u32,
                7u32,
                262203u32,
                8u32,
                580u32,
                7u32,
                262203u32,
                8u32,
                582u32,
                7u32,
                262203u32,
                586u32,
                587u32,
                7u32,
                262205u32,
                558u32,
                561u32,
                560u32,
                262248u32,
                7u32,
                562u32,
                561u32,
                196670u32,
                556u32,
                562u32,
                262205u32,
                165u32,
                565u32,
                167u32,
                458831u32,
                564u32,
                566u32,
                565u32,
                565u32,
                0u32,
                1u32,
                262268u32,
                7u32,
                567u32,
                566u32,
                196670u32,
                563u32,
                567u32,
                327745u32,
                574u32,
                575u32,
                573u32,
                102u32,
                262205u32,
                29u32,
                576u32,
                575u32,
                327851u32,
                28u32,
                577u32,
                576u32,
                46u32,
                196855u32,
                579u32,
                0u32,
                262394u32,
                577u32,
                578u32,
                579u32,
                131320u32,
                578u32,
                262205u32,
                7u32,
                581u32,
                563u32,
                196670u32,
                580u32,
                581u32,
                262205u32,
                7u32,
                583u32,
                556u32,
                196670u32,
                582u32,
                583u32,
                393273u32,
                2u32,
                584u32,
                20u32,
                580u32,
                582u32,
                131321u32,
                579u32,
                131320u32,
                579u32,
                262368u32,
                171u32,
                171u32,
                585u32,
                327745u32,
                31u32,
                588u32,
                563u32,
                30u32,
                262205u32,
                6u32,
                589u32,
                588u32,
                327745u32,
                31u32,
                590u32,
                556u32,
                46u32,
                262205u32,
                6u32,
                591u32,
                590u32,
                327812u32,
                6u32,
                592u32,
                589u32,
                591u32,
                327745u32,
                31u32,
                593u32,
                563u32,
                46u32,
                262205u32,
                6u32,
                594u32,
                593u32,
                327808u32,
                6u32,
                595u32,
                592u32,
                594u32,
                393281u32,
                53u32,
                596u32,
                43u32,
                24u32,
                595u32,
                262205u32,
                29u32,
                597u32,
                596u32,
                393281u32,
                598u32,
                599u32,
                573u32,
                24u32,
                597u32,
                262205u32,
                568u32,
                600u32,
                599u32,
                196670u32,
                587u32,
                600u32,
                262205u32,
                558u32,
                601u32,
                560u32,
                262205u32,
                7u32,
                602u32,
                563u32,
                262205u32,
                568u32,
                603u32,
                587u32,
                262243u32,
                601u32,
                602u32,
                603u32,
                65789u32,
                65592u32,
                327734u32,
                2u32,
                12u32,
                0u32,
                9u32,
                196663u32,
                8u32,
                10u32,
                196663u32,
                8u32,
                11u32,
                131320u32,
                13u32,
                262203u32,
                8u32,
                22u32,
                7u32,
                262203u32,
                8u32,
                96u32,
                7u32,
                262203u32,
                8u32,
                100u32,
                7u32,
                262203u32,
                105u32,
                106u32,
                7u32,
                262203u32,
                105u32,
                132u32,
                7u32,
                262205u32,
                7u32,
                23u32,
                10u32,
                327808u32,
                7u32,
                27u32,
                23u32,
                26u32,
                196670u32,
                22u32,
                27u32,
                327745u32,
                31u32,
                32u32,
                22u32,
                30u32,
                262205u32,
                6u32,
                33u32,
                32u32,
                327745u32,
                31u32,
                34u32,
                11u32,
                30u32,
                262205u32,
                6u32,
                35u32,
                34u32,
                327855u32,
                28u32,
                36u32,
                33u32,
                35u32,
                262312u32,
                28u32,
                37u32,
                36u32,
                196855u32,
                39u32,
                0u32,
                262394u32,
                37u32,
                38u32,
                39u32,
                131320u32,
                38u32,
                327745u32,
                31u32,
                44u32,
                22u32,
                30u32,
                262205u32,
                6u32,
                45u32,
                44u32,
                327745u32,
                31u32,
                47u32,
                11u32,
                46u32,
                262205u32,
                6u32,
                48u32,
                47u32,
                327812u32,
                6u32,
                49u32,
                45u32,
                48u32,
                327745u32,
                31u32,
                50u32,
                22u32,
                46u32,
                262205u32,
                6u32,
                51u32,
                50u32,
                327808u32,
                6u32,
                52u32,
                49u32,
                51u32,
                393281u32,
                53u32,
                54u32,
                43u32,
                24u32,
                52u32,
                458981u32,
                29u32,
                55u32,
                54u32,
                30u32,
                46u32,
                30u32,
                327850u32,
                28u32,
                56u32,
                55u32,
                46u32,
                131321u32,
                39u32,
                131320u32,
                39u32,
                458997u32,
                28u32,
                57u32,
                36u32,
                13u32,
                56u32,
                38u32,
                196855u32,
                59u32,
                0u32,
                262394u32,
                57u32,
                58u32,
                70u32,
                131320u32,
                58u32,
                327745u32,
                31u32,
                60u32,
                10u32,
                30u32,
                262205u32,
                6u32,
                61u32,
                60u32,
                327745u32,
                31u32,
                62u32,
                11u32,
                46u32,
                262205u32,
                6u32,
                63u32,
                62u32,
                327812u32,
                6u32,
                64u32,
                61u32,
                63u32,
                327745u32,
                31u32,
                65u32,
                10u32,
                46u32,
                262205u32,
                6u32,
                66u32,
                65u32,
                327808u32,
                6u32,
                67u32,
                64u32,
                66u32,
                393281u32,
                53u32,
                68u32,
                43u32,
                24u32,
                67u32,
                458981u32,
                29u32,
                69u32,
                68u32,
                30u32,
                46u32,
                46u32,
                131321u32,
                59u32,
                131320u32,
                70u32,
                327745u32,
                31u32,
                71u32,
                22u32,
                30u32,
                262205u32,
                6u32,
                72u32,
                71u32,
                327745u32,
                31u32,
                73u32,
                11u32,
                46u32,
                262205u32,
                6u32,
                74u32,
                73u32,
                327812u32,
                6u32,
                75u32,
                72u32,
                74u32,
                327745u32,
                31u32,
                76u32,
                22u32,
                46u32,
                262205u32,
                6u32,
                77u32,
                76u32,
                327808u32,
                6u32,
                78u32,
                75u32,
                77u32,
                393281u32,
                53u32,
                79u32,
                43u32,
                24u32,
                78u32,
                458981u32,
                29u32,
                80u32,
                79u32,
                30u32,
                46u32,
                30u32,
                327850u32,
                28u32,
                82u32,
                80u32,
                81u32,
                196855u32,
                84u32,
                0u32,
                262394u32,
                82u32,
                83u32,
                95u32,
                131320u32,
                83u32,
                327745u32,
                31u32,
                85u32,
                10u32,
                30u32,
                262205u32,
                6u32,
                86u32,
                85u32,
                327745u32,
                31u32,
                87u32,
                11u32,
                46u32,
                262205u32,
                6u32,
                88u32,
                87u32,
                327812u32,
                6u32,
                89u32,
                86u32,
                88u32,
                327745u32,
                31u32,
                90u32,
                10u32,
                46u32,
                262205u32,
                6u32,
                91u32,
                90u32,
                327808u32,
                6u32,
                92u32,
                89u32,
                91u32,
                393281u32,
                53u32,
                93u32,
                43u32,
                24u32,
                92u32,
                458981u32,
                29u32,
                94u32,
                93u32,
                30u32,
                46u32,
                81u32,
                131321u32,
                84u32,
                131320u32,
                95u32,
                262205u32,
                7u32,
                97u32,
                10u32,
                327808u32,
                7u32,
                99u32,
                97u32,
                98u32,
                196670u32,
                96u32,
                99u32,
                262205u32,
                7u32,
                101u32,
                10u32,
                327808u32,
                7u32,
                104u32,
                101u32,
                103u32,
                196670u32,
                100u32,
                104u32,
                327745u32,
                31u32,
                107u32,
                96u32,
                46u32,
                262205u32,
                6u32,
                108u32,
                107u32,
                327855u32,
                28u32,
                109u32,
                108u32,
                24u32,
                196855u32,
                111u32,
                0u32,
                262394u32,
                109u32,
                110u32,
                111u32,
                131320u32,
                110u32,
                327745u32,
                31u32,
                112u32,
                96u32,
                30u32,
                262205u32,
                6u32,
                113u32,
                112u32,
                327745u32,
                31u32,
                114u32,
                11u32,
                30u32,
                262205u32,
                6u32,
                115u32,
                114u32,
                327857u32,
                28u32,
                116u32,
                113u32,
                115u32,
                131321u32,
                111u32,
                131320u32,
                111u32,
                458997u32,
                28u32,
                117u32,
                109u32,
                95u32,
                116u32,
                110u32,
                196855u32,
                119u32,
                0u32,
                262394u32,
                117u32,
                118u32,
                119u32,
                131320u32,
                118u32,
                327745u32,
                31u32,
                120u32,
                96u32,
                30u32,
                262205u32,
                6u32,
                121u32,
                120u32,
                327745u32,
                31u32,
                122u32,
                11u32,
                46u32,
                262205u32,
                6u32,
                123u32,
                122u32,
                327812u32,
                6u32,
                124u32,
                121u32,
                123u32,
                327745u32,
                31u32,
                125u32,
                96u32,
                46u32,
                262205u32,
                6u32,
                126u32,
                125u32,
                327808u32,
                6u32,
                127u32,
                124u32,
                126u32,
                393281u32,
                53u32,
                128u32,
                43u32,
                24u32,
                127u32,
                458981u32,
                29u32,
                129u32,
                128u32,
                30u32,
                46u32,
                30u32,
                327850u32,
                28u32,
                130u32,
                129u32,
                46u32,
                131321u32,
                119u32,
                131320u32,
                119u32,
                458997u32,
                28u32,
                131u32,
                117u32,
                111u32,
                130u32,
                118u32,
                196670u32,
                106u32,
                131u32,
                327745u32,
                31u32,
                133u32,
                100u32,
                46u32,
                262205u32,
                6u32,
                134u32,
                133u32,
                327745u32,
                31u32,
                135u32,
                11u32,
                46u32,
                262205u32,
                6u32,
                136u32,
                135u32,
                327857u32,
                28u32,
                137u32,
                134u32,
                136u32,
                196855u32,
                139u32,
                0u32,
                262394u32,
                137u32,
                138u32,
                139u32,
                131320u32,
                138u32,
                327745u32,
                31u32,
                140u32,
                100u32,
                30u32,
                262205u32,
                6u32,
                141u32,
                140u32,
                327745u32,
                31u32,
                142u32,
                11u32,
                30u32,
                262205u32,
                6u32,
                143u32,
                142u32,
                327857u32,
                28u32,
                144u32,
                141u32,
                143u32,
                131321u32,
                139u32,
                131320u32,
                139u32,
                458997u32,
                28u32,
                145u32,
                137u32,
                119u32,
                144u32,
                138u32,
                196855u32,
                147u32,
                0u32,
                262394u32,
                145u32,
                146u32,
                147u32,
                131320u32,
                146u32,
                327745u32,
                31u32,
                148u32,
                100u32,
                30u32,
                262205u32,
                6u32,
                149u32,
                148u32,
                327745u32,
                31u32,
                150u32,
                11u32,
                46u32,
                262205u32,
                6u32,
                151u32,
                150u32,
                327812u32,
                6u32,
                152u32,
                149u32,
                151u32,
                327745u32,
                31u32,
                153u32,
                100u32,
                46u32,
                262205u32,
                6u32,
                154u32,
                153u32,
                327808u32,
                6u32,
                155u32,
                152u32,
                154u32,
                393281u32,
                53u32,
                156u32,
                43u32,
                24u32,
                155u32,
                458981u32,
                29u32,
                157u32,
                156u32,
                30u32,
                46u32,
                30u32,
                327850u32,
                28u32,
                158u32,
                157u32,
                46u32,
                131321u32,
                147u32,
                131320u32,
                147u32,
                458997u32,
                28u32,
                159u32,
                145u32,
                139u32,
                158u32,
                146u32,
                196670u32,
                132u32,
                159u32,
                262205u32,
                28u32,
                160u32,
                106u32,
                262205u32,
                28u32,
                161u32,
                132u32,
                327847u32,
                28u32,
                162u32,
                160u32,
                161u32,
                196855u32,
                164u32,
                0u32,
                262394u32,
                162u32,
                163u32,
                197u32,
                131320u32,
                163u32,
                327745u32,
                168u32,
                169u32,
                167u32,
                46u32,
                262205u32,
                29u32,
                170u32,
                169u32,
                327817u32,
                29u32,
                172u32,
                170u32,
                171u32,
                327850u32,
                28u32,
                173u32,
                172u32,
                46u32,
                196855u32,
                175u32,
                0u32,
                262394u32,
                173u32,
                174u32,
                186u32,
                131320u32,
                174u32,
                327745u32,
                31u32,
                176u32,
                10u32,
                30u32,
                262205u32,
                6u32,
                177u32,
                176u32,
                327745u32,
                31u32,
                178u32,
                11u32,
                46u32,
                262205u32,
                6u32,
                179u32,
                178u32,
                327812u32,
                6u32,
                180u32,
                177u32,
                179u32,
                327745u32,
                31u32,
                181u32,
                10u32,
                46u32,
                262205u32,
                6u32,
                182u32,
                181u32,
                327808u32,
                6u32,
                183u32,
                180u32,
                182u32,
                393281u32,
                53u32,
                184u32,
                43u32,
                24u32,
                183u32,
                458981u32,
                29u32,
                185u32,
                184u32,
                30u32,
                46u32,
                46u32,
                131321u32,
                175u32,
                131320u32,
                186u32,
                327745u32,
                31u32,
                187u32,
                10u32,
                30u32,
                262205u32,
                6u32,
                188u32,
                187u32,
                327745u32,
                31u32,
                189u32,
                11u32,
                46u32,
                262205u32,
                6u32,
                190u32,
                189u32,
                327812u32,
                6u32,
                191u32,
                188u32,
                190u32,
                327745u32,
                31u32,
                192u32,
                10u32,
                46u32,
                262205u32,
                6u32,
                193u32,
                192u32,
                327808u32,
                6u32,
                194u32,
                191u32,
                193u32,
                393281u32,
                53u32,
                195u32,
                43u32,
                24u32,
                194u32,
                458981u32,
                29u32,
                196u32,
                195u32,
                30u32,
                46u32,
                46u32,
                131321u32,
                175u32,
                131320u32,
                175u32,
                131321u32,
                164u32,
                131320u32,
                197u32,
                262205u32,
                28u32,
                198u32,
                106u32,
                196855u32,
                200u32,
                0u32,
                262394u32,
                198u32,
                199u32,
                211u32,
                131320u32,
                199u32,
                327745u32,
                31u32,
                201u32,
                10u32,
                30u32,
                262205u32,
                6u32,
                202u32,
                201u32,
                327745u32,
                31u32,
                203u32,
                11u32,
                46u32,
                262205u32,
                6u32,
                204u32,
                203u32,
                327812u32,
                6u32,
                205u32,
                202u32,
                204u32,
                327745u32,
                31u32,
                206u32,
                10u32,
                46u32,
                262205u32,
                6u32,
                207u32,
                206u32,
                327808u32,
                6u32,
                208u32,
                205u32,
                207u32,
                393281u32,
                53u32,
                209u32,
                43u32,
                24u32,
                208u32,
                458981u32,
                29u32,
                210u32,
                209u32,
                30u32,
                46u32,
                46u32,
                131321u32,
                200u32,
                131320u32,
                211u32,
                262205u32,
                28u32,
                212u32,
                132u32,
                196855u32,
                214u32,
                0u32,
                262394u32,
                212u32,
                213u32,
                214u32,
                131320u32,
                213u32,
                327745u32,
                31u32,
                215u32,
                10u32,
                30u32,
                262205u32,
                6u32,
                216u32,
                215u32,
                327745u32,
                31u32,
                217u32,
                11u32,
                46u32,
                262205u32,
                6u32,
                218u32,
                217u32,
                327812u32,
                6u32,
                219u32,
                216u32,
                218u32,
                327745u32,
                31u32,
                220u32,
                10u32,
                46u32,
                262205u32,
                6u32,
                221u32,
                220u32,
                327808u32,
                6u32,
                222u32,
                219u32,
                221u32,
                393281u32,
                53u32,
                223u32,
                43u32,
                24u32,
                222u32,
                458981u32,
                29u32,
                224u32,
                223u32,
                30u32,
                46u32,
                46u32,
                131321u32,
                214u32,
                131320u32,
                214u32,
                131321u32,
                200u32,
                131320u32,
                200u32,
                131321u32,
                164u32,
                131320u32,
                164u32,
                131321u32,
                84u32,
                131320u32,
                84u32,
                131321u32,
                59u32,
                131320u32,
                59u32,
                65789u32,
                65592u32,
                327734u32,
                2u32,
                16u32,
                0u32,
                9u32,
                196663u32,
                8u32,
                14u32,
                196663u32,
                8u32,
                15u32,
                131320u32,
                17u32,
                262203u32,
                8u32,
                225u32,
                7u32,
                262203u32,
                8u32,
                261u32,
                7u32,
                262203u32,
                8u32,
                264u32,
                7u32,
                262203u32,
                105u32,
                267u32,
                7u32,
                262203u32,
                105u32,
                293u32,
                7u32,
                262203u32,
                8u32,
                381u32,
                7u32,
                262203u32,
                8u32,
                385u32,
                7u32,
                262203u32,
                105u32,
                389u32,
                7u32,
                262203u32,
                105u32,
                415u32,
                7u32,
                262205u32,
                7u32,
                226u32,
                14u32,
                327808u32,
                7u32,
                227u32,
                226u32,
                26u32,
                196670u32,
                225u32,
                227u32,
                327745u32,
                31u32,
                228u32,
                225u32,
                30u32,
                262205u32,
                6u32,
                229u32,
                228u32,
                327745u32,
                31u32,
                230u32,
                15u32,
                30u32,
                262205u32,
                6u32,
                231u32,
                230u32,
                327855u32,
                28u32,
                232u32,
                229u32,
                231u32,
                262312u32,
                28u32,
                233u32,
                232u32,
                196855u32,
                235u32,
                0u32,
                262394u32,
                233u32,
                234u32,
                235u32,
                131320u32,
                234u32,
                327745u32,
                31u32,
                236u32,
                225u32,
                30u32,
                262205u32,
                6u32,
                237u32,
                236u32,
                327745u32,
                31u32,
                238u32,
                15u32,
                46u32,
                262205u32,
                6u32,
                239u32,
                238u32,
                327812u32,
                6u32,
                240u32,
                237u32,
                239u32,
                327745u32,
                31u32,
                241u32,
                225u32,
                46u32,
                262205u32,
                6u32,
                242u32,
                241u32,
                327808u32,
                6u32,
                243u32,
                240u32,
                242u32,
                393281u32,
                53u32,
                244u32,
                43u32,
                24u32,
                243u32,
                458981u32,
                29u32,
                245u32,
                244u32,
                30u32,
                46u32,
                81u32,
                327850u32,
                28u32,
                246u32,
                245u32,
                46u32,
                131321u32,
                235u32,
                131320u32,
                235u32,
                458997u32,
                28u32,
                247u32,
                232u32,
                17u32,
                246u32,
                234u32,
                196855u32,
                249u32,
                0u32,
                262394u32,
                247u32,
                248u32,
                260u32,
                131320u32,
                248u32,
                327745u32,
                31u32,
                250u32,
                14u32,
                30u32,
                262205u32,
                6u32,
                251u32,
                250u32,
                327745u32,
                31u32,
                252u32,
                15u32,
                46u32,
                262205u32,
                6u32,
                253u32,
                252u32,
                327812u32,
                6u32,
                254u32,
                251u32,
                253u32,
                327745u32,
                31u32,
                255u32,
                14u32,
                46u32,
                262205u32,
                6u32,
                256u32,
                255u32,
                327808u32,
                6u32,
                257u32,
                254u32,
                256u32,
                393281u32,
                53u32,
                258u32,
                43u32,
                24u32,
                257u32,
                458981u32,
                29u32,
                259u32,
                258u32,
                30u32,
                46u32,
                46u32,
                131321u32,
                249u32,
                131320u32,
                260u32,
                262205u32,
                7u32,
                262u32,
                14u32,
                327808u32,
                7u32,
                263u32,
                262u32,
                98u32,
                196670u32,
                261u32,
                263u32,
                262205u32,
                7u32,
                265u32,
                14u32,
                327808u32,
                7u32,
                266u32,
                265u32,
                103u32,
                196670u32,
                264u32,
                266u32,
                327745u32,
                31u32,
                268u32,
                261u32,
                46u32,
                262205u32,
                6u32,
                269u32,
                268u32,
                327855u32,
                28u32,
                270u32,
                269u32,
                24u32,
                196855u32,
                272u32,
                0u32,
                262394u32,
                270u32,
                271u32,
                272u32,
                131320u32,
                271u32,
                327745u32,
                31u32,
                273u32,
                261u32,
                30u32,
                262205u32,
                6u32,
                274u32,
                273u32,
                327745u32,
                31u32,
                275u32,
                15u32,
                30u32,
                262205u32,
                6u32,
                276u32,
                275u32,
                327857u32,
                28u32,
                277u32,
                274u32,
                276u32,
                131321u32,
                272u32,
                131320u32,
                272u32,
                458997u32,
                28u32,
                278u32,
                270u32,
                260u32,
                277u32,
                271u32,
                196855u32,
                280u32,
                0u32,
                262394u32,
                278u32,
                279u32,
                280u32,
                131320u32,
                279u32,
                327745u32,
                31u32,
                281u32,
                261u32,
                30u32,
                262205u32,
                6u32,
                282u32,
                281u32,
                327745u32,
                31u32,
                283u32,
                15u32,
                46u32,
                262205u32,
                6u32,
                284u32,
                283u32,
                327812u32,
                6u32,
                285u32,
                282u32,
                284u32,
                327745u32,
                31u32,
                286u32,
                261u32,
                46u32,
                262205u32,
                6u32,
                287u32,
                286u32,
                327808u32,
                6u32,
                288u32,
                285u32,
                287u32,
                393281u32,
                53u32,
                289u32,
                43u32,
                24u32,
                288u32,
                458981u32,
                29u32,
                290u32,
                289u32,
                30u32,
                46u32,
                81u32,
                327850u32,
                28u32,
                291u32,
                290u32,
                46u32,
                131321u32,
                280u32,
                131320u32,
                280u32,
                458997u32,
                28u32,
                292u32,
                278u32,
                272u32,
                291u32,
                279u32,
                196670u32,
                267u32,
                292u32,
                327745u32,
                31u32,
                294u32,
                264u32,
                46u32,
                262205u32,
                6u32,
                295u32,
                294u32,
                327745u32,
                31u32,
                296u32,
                15u32,
                46u32,
                262205u32,
                6u32,
                297u32,
                296u32,
                327857u32,
                28u32,
                298u32,
                295u32,
                297u32,
                196855u32,
                300u32,
                0u32,
                262394u32,
                298u32,
                299u32,
                300u32,
                131320u32,
                299u32,
                327745u32,
                31u32,
                301u32,
                264u32,
                30u32,
                262205u32,
                6u32,
                302u32,
                301u32,
                327745u32,
                31u32,
                303u32,
                15u32,
                30u32,
                262205u32,
                6u32,
                304u32,
                303u32,
                327857u32,
                28u32,
                305u32,
                302u32,
                304u32,
                131321u32,
                300u32,
                131320u32,
                300u32,
                458997u32,
                28u32,
                306u32,
                298u32,
                280u32,
                305u32,
                299u32,
                196855u32,
                308u32,
                0u32,
                262394u32,
                306u32,
                307u32,
                308u32,
                131320u32,
                307u32,
                327745u32,
                31u32,
                309u32,
                264u32,
                30u32,
                262205u32,
                6u32,
                310u32,
                309u32,
                327745u32,
                31u32,
                311u32,
                15u32,
                46u32,
                262205u32,
                6u32,
                312u32,
                311u32,
                327812u32,
                6u32,
                313u32,
                310u32,
                312u32,
                327745u32,
                31u32,
                314u32,
                264u32,
                46u32,
                262205u32,
                6u32,
                315u32,
                314u32,
                327808u32,
                6u32,
                316u32,
                313u32,
                315u32,
                393281u32,
                53u32,
                317u32,
                43u32,
                24u32,
                316u32,
                458981u32,
                29u32,
                318u32,
                317u32,
                30u32,
                46u32,
                81u32,
                327850u32,
                28u32,
                319u32,
                318u32,
                46u32,
                131321u32,
                308u32,
                131320u32,
                308u32,
                458997u32,
                28u32,
                320u32,
                306u32,
                300u32,
                319u32,
                307u32,
                196670u32,
                293u32,
                320u32,
                262205u32,
                28u32,
                321u32,
                267u32,
                262205u32,
                28u32,
                322u32,
                293u32,
                327847u32,
                28u32,
                323u32,
                321u32,
                322u32,
                196855u32,
                325u32,
                0u32,
                262394u32,
                323u32,
                324u32,
                353u32,
                131320u32,
                324u32,
                327745u32,
                168u32,
                326u32,
                167u32,
                46u32,
                262205u32,
                29u32,
                327u32,
                326u32,
                327817u32,
                29u32,
                328u32,
                327u32,
                171u32,
                327850u32,
                28u32,
                329u32,
                328u32,
                46u32,
                196855u32,
                331u32,
                0u32,
                262394u32,
                329u32,
                330u32,
                342u32,
                131320u32,
                330u32,
                327745u32,
                31u32,
                332u32,
                14u32,
                30u32,
                262205u32,
                6u32,
                333u32,
                332u32,
                327745u32,
                31u32,
                334u32,
                15u32,
                46u32,
                262205u32,
                6u32,
                335u32,
                334u32,
                327812u32,
                6u32,
                336u32,
                333u32,
                335u32,
                327745u32,
                31u32,
                337u32,
                14u32,
                46u32,
                262205u32,
                6u32,
                338u32,
                337u32,
                327808u32,
                6u32,
                339u32,
                336u32,
                338u32,
                393281u32,
                53u32,
                340u32,
                43u32,
                24u32,
                339u32,
                458981u32,
                29u32,
                341u32,
                340u32,
                30u32,
                46u32,
                46u32,
                131321u32,
                331u32,
                131320u32,
                342u32,
                327745u32,
                31u32,
                343u32,
                14u32,
                30u32,
                262205u32,
                6u32,
                344u32,
                343u32,
                327745u32,
                31u32,
                345u32,
                15u32,
                46u32,
                262205u32,
                6u32,
                346u32,
                345u32,
                327812u32,
                6u32,
                347u32,
                344u32,
                346u32,
                327745u32,
                31u32,
                348u32,
                14u32,
                46u32,
                262205u32,
                6u32,
                349u32,
                348u32,
                327808u32,
                6u32,
                350u32,
                347u32,
                349u32,
                393281u32,
                53u32,
                351u32,
                43u32,
                24u32,
                350u32,
                458981u32,
                29u32,
                352u32,
                351u32,
                30u32,
                46u32,
                46u32,
                131321u32,
                331u32,
                131320u32,
                331u32,
                131321u32,
                325u32,
                131320u32,
                353u32,
                262205u32,
                28u32,
                354u32,
                267u32,
                196855u32,
                356u32,
                0u32,
                262394u32,
                354u32,
                355u32,
                367u32,
                131320u32,
                355u32,
                327745u32,
                31u32,
                357u32,
                14u32,
                30u32,
                262205u32,
                6u32,
                358u32,
                357u32,
                327745u32,
                31u32,
                359u32,
                15u32,
                46u32,
                262205u32,
                6u32,
                360u32,
                359u32,
                327812u32,
                6u32,
                361u32,
                358u32,
                360u32,
                327745u32,
                31u32,
                362u32,
                14u32,
                46u32,
                262205u32,
                6u32,
                363u32,
                362u32,
                327808u32,
                6u32,
                364u32,
                361u32,
                363u32,
                393281u32,
                53u32,
                365u32,
                43u32,
                24u32,
                364u32,
                458981u32,
                29u32,
                366u32,
                365u32,
                30u32,
                46u32,
                46u32,
                131321u32,
                356u32,
                131320u32,
                367u32,
                262205u32,
                28u32,
                368u32,
                293u32,
                196855u32,
                370u32,
                0u32,
                262394u32,
                368u32,
                369u32,
                370u32,
                131320u32,
                369u32,
                327745u32,
                31u32,
                371u32,
                14u32,
                30u32,
                262205u32,
                6u32,
                372u32,
                371u32,
                327745u32,
                31u32,
                373u32,
                15u32,
                46u32,
                262205u32,
                6u32,
                374u32,
                373u32,
                327812u32,
                6u32,
                375u32,
                372u32,
                374u32,
                327745u32,
                31u32,
                376u32,
                14u32,
                46u32,
                262205u32,
                6u32,
                377u32,
                376u32,
                327808u32,
                6u32,
                378u32,
                375u32,
                377u32,
                393281u32,
                53u32,
                379u32,
                43u32,
                24u32,
                378u32,
                458981u32,
                29u32,
                380u32,
                379u32,
                30u32,
                46u32,
                46u32,
                131321u32,
                370u32,
                131320u32,
                370u32,
                131321u32,
                356u32,
                131320u32,
                356u32,
                131321u32,
                325u32,
                131320u32,
                325u32,
                262205u32,
                7u32,
                382u32,
                14u32,
                327808u32,
                7u32,
                384u32,
                382u32,
                383u32,
                196670u32,
                381u32,
                384u32,
                262205u32,
                7u32,
                386u32,
                14u32,
                327808u32,
                7u32,
                388u32,
                386u32,
                387u32,
                196670u32,
                385u32,
                388u32,
                327745u32,
                31u32,
                390u32,
                261u32,
                46u32,
                262205u32,
                6u32,
                391u32,
                390u32,
                327855u32,
                28u32,
                392u32,
                391u32,
                24u32,
                196855u32,
                394u32,
                0u32,
                262394u32,
                392u32,
                393u32,
                394u32,
                131320u32,
                393u32,
                327745u32,
                31u32,
                395u32,
                261u32,
                30u32,
                262205u32,
                6u32,
                396u32,
                395u32,
                327745u32,
                31u32,
                397u32,
                15u32,
                30u32,
                262205u32,
                6u32,
                398u32,
                397u32,
                327857u32,
                28u32,
                399u32,
                396u32,
                398u32,
                131321u32,
                394u32,
                131320u32,
                394u32,
                458997u32,
                28u32,
                400u32,
                392u32,
                325u32,
                399u32,
                393u32,
                196855u32,
                402u32,
                0u32,
                262394u32,
                400u32,
                401u32,
                402u32,
                131320u32,
                401u32,
                327745u32,
                31u32,
                403u32,
                381u32,
                30u32,
                262205u32,
                6u32,
                404u32,
                403u32,
                327745u32,
                31u32,
                405u32,
                15u32,
                46u32,
                262205u32,
                6u32,
                406u32,
                405u32,
                327812u32,
                6u32,
                407u32,
                404u32,
                406u32,
                327745u32,
                31u32,
                408u32,
                381u32,
                46u32,
                262205u32,
                6u32,
                409u32,
                408u32,
                327808u32,
                6u32,
                410u32,
                407u32,
                409u32,
                393281u32,
                53u32,
                411u32,
                43u32,
                24u32,
                410u32,
                458981u32,
                29u32,
                412u32,
                411u32,
                30u32,
                46u32,
                81u32,
                327850u32,
                28u32,
                413u32,
                412u32,
                46u32,
                131321u32,
                402u32,
                131320u32,
                402u32,
                458997u32,
                28u32,
                414u32,
                400u32,
                394u32,
                413u32,
                401u32,
                196670u32,
                389u32,
                414u32,
                327745u32,
                31u32,
                416u32,
                264u32,
                46u32,
                262205u32,
                6u32,
                417u32,
                416u32,
                327745u32,
                31u32,
                418u32,
                15u32,
                46u32,
                262205u32,
                6u32,
                419u32,
                418u32,
                327857u32,
                28u32,
                420u32,
                417u32,
                419u32,
                196855u32,
                422u32,
                0u32,
                262394u32,
                420u32,
                421u32,
                422u32,
                131320u32,
                421u32,
                327745u32,
                31u32,
                423u32,
                264u32,
                30u32,
                262205u32,
                6u32,
                424u32,
                423u32,
                327745u32,
                31u32,
                425u32,
                15u32,
                30u32,
                262205u32,
                6u32,
                426u32,
                425u32,
                327857u32,
                28u32,
                427u32,
                424u32,
                426u32,
                131321u32,
                422u32,
                131320u32,
                422u32,
                458997u32,
                28u32,
                428u32,
                420u32,
                402u32,
                427u32,
                421u32,
                196855u32,
                430u32,
                0u32,
                262394u32,
                428u32,
                429u32,
                430u32,
                131320u32,
                429u32,
                327745u32,
                31u32,
                431u32,
                385u32,
                30u32,
                262205u32,
                6u32,
                432u32,
                431u32,
                327745u32,
                31u32,
                433u32,
                15u32,
                46u32,
                262205u32,
                6u32,
                434u32,
                433u32,
                327812u32,
                6u32,
                435u32,
                432u32,
                434u32,
                327745u32,
                31u32,
                436u32,
                385u32,
                46u32,
                262205u32,
                6u32,
                437u32,
                436u32,
                327808u32,
                6u32,
                438u32,
                435u32,
                437u32,
                393281u32,
                53u32,
                439u32,
                43u32,
                24u32,
                438u32,
                458981u32,
                29u32,
                440u32,
                439u32,
                30u32,
                46u32,
                81u32,
                327850u32,
                28u32,
                441u32,
                440u32,
                46u32,
                131321u32,
                430u32,
                131320u32,
                430u32,
                458997u32,
                28u32,
                442u32,
                428u32,
                422u32,
                441u32,
                429u32,
                196670u32,
                415u32,
                442u32,
                262205u32,
                28u32,
                443u32,
                389u32,
                262205u32,
                28u32,
                444u32,
                415u32,
                327847u32,
                28u32,
                445u32,
                443u32,
                444u32,
                196855u32,
                447u32,
                0u32,
                262394u32,
                445u32,
                446u32,
                475u32,
                131320u32,
                446u32,
                327745u32,
                168u32,
                448u32,
                167u32,
                46u32,
                262205u32,
                29u32,
                449u32,
                448u32,
                327817u32,
                29u32,
                450u32,
                449u32,
                171u32,
                327850u32,
                28u32,
                451u32,
                450u32,
                46u32,
                196855u32,
                453u32,
                0u32,
                262394u32,
                451u32,
                452u32,
                464u32,
                131320u32,
                452u32,
                327745u32,
                31u32,
                454u32,
                14u32,
                30u32,
                262205u32,
                6u32,
                455u32,
                454u32,
                327745u32,
                31u32,
                456u32,
                15u32,
                46u32,
                262205u32,
                6u32,
                457u32,
                456u32,
                327812u32,
                6u32,
                458u32,
                455u32,
                457u32,
                327745u32,
                31u32,
                459u32,
                14u32,
                46u32,
                262205u32,
                6u32,
                460u32,
                459u32,
                327808u32,
                6u32,
                461u32,
                458u32,
                460u32,
                393281u32,
                53u32,
                462u32,
                43u32,
                24u32,
                461u32,
                458981u32,
                29u32,
                463u32,
                462u32,
                30u32,
                46u32,
                46u32,
                131321u32,
                453u32,
                131320u32,
                464u32,
                327745u32,
                31u32,
                465u32,
                14u32,
                30u32,
                262205u32,
                6u32,
                466u32,
                465u32,
                327745u32,
                31u32,
                467u32,
                15u32,
                46u32,
                262205u32,
                6u32,
                468u32,
                467u32,
                327812u32,
                6u32,
                469u32,
                466u32,
                468u32,
                327745u32,
                31u32,
                470u32,
                14u32,
                46u32,
                262205u32,
                6u32,
                471u32,
                470u32,
                327808u32,
                6u32,
                472u32,
                469u32,
                471u32,
                393281u32,
                53u32,
                473u32,
                43u32,
                24u32,
                472u32,
                458981u32,
                29u32,
                474u32,
                473u32,
                30u32,
                46u32,
                46u32,
                131321u32,
                453u32,
                131320u32,
                453u32,
                131321u32,
                447u32,
                131320u32,
                475u32,
                262205u32,
                28u32,
                476u32,
                389u32,
                196855u32,
                478u32,
                0u32,
                262394u32,
                476u32,
                477u32,
                489u32,
                131320u32,
                477u32,
                327745u32,
                31u32,
                479u32,
                14u32,
                30u32,
                262205u32,
                6u32,
                480u32,
                479u32,
                327745u32,
                31u32,
                481u32,
                15u32,
                46u32,
                262205u32,
                6u32,
                482u32,
                481u32,
                327812u32,
                6u32,
                483u32,
                480u32,
                482u32,
                327745u32,
                31u32,
                484u32,
                14u32,
                46u32,
                262205u32,
                6u32,
                485u32,
                484u32,
                327808u32,
                6u32,
                486u32,
                483u32,
                485u32,
                393281u32,
                53u32,
                487u32,
                43u32,
                24u32,
                486u32,
                458981u32,
                29u32,
                488u32,
                487u32,
                30u32,
                46u32,
                46u32,
                131321u32,
                478u32,
                131320u32,
                489u32,
                262205u32,
                28u32,
                490u32,
                415u32,
                196855u32,
                492u32,
                0u32,
                262394u32,
                490u32,
                491u32,
                492u32,
                131320u32,
                491u32,
                327745u32,
                31u32,
                493u32,
                14u32,
                30u32,
                262205u32,
                6u32,
                494u32,
                493u32,
                327745u32,
                31u32,
                495u32,
                15u32,
                46u32,
                262205u32,
                6u32,
                496u32,
                495u32,
                327812u32,
                6u32,
                497u32,
                494u32,
                496u32,
                327745u32,
                31u32,
                498u32,
                14u32,
                46u32,
                262205u32,
                6u32,
                499u32,
                498u32,
                327808u32,
                6u32,
                500u32,
                497u32,
                499u32,
                393281u32,
                53u32,
                501u32,
                43u32,
                24u32,
                500u32,
                458981u32,
                29u32,
                502u32,
                501u32,
                30u32,
                46u32,
                46u32,
                131321u32,
                492u32,
                131320u32,
                492u32,
                131321u32,
                478u32,
                131320u32,
                478u32,
                131321u32,
                447u32,
                131320u32,
                447u32,
                131321u32,
                249u32,
                131320u32,
                249u32,
                65789u32,
                65592u32,
                327734u32,
                2u32,
                20u32,
                0u32,
                9u32,
                196663u32,
                8u32,
                18u32,
                196663u32,
                8u32,
                19u32,
                131320u32,
                21u32,
                262203u32,
                520u32,
                521u32,
                7u32,
                262203u32,
                8u32,
                536u32,
                7u32,
                262203u32,
                8u32,
                538u32,
                7u32,
                262203u32,
                8u32,
                546u32,
                7u32,
                262203u32,
                8u32,
                548u32,
                7u32,
                327745u32,
                31u32,
                503u32,
                18u32,
                46u32,
                262205u32,
                6u32,
                504u32,
                503u32,
                327745u32,
                31u32,
                505u32,
                19u32,
                46u32,
                262205u32,
                6u32,
                506u32,
                505u32,
                327855u32,
                28u32,
                507u32,
                504u32,
                506u32,
                262312u32,
                28u32,
                508u32,
                507u32,
                196855u32,
                510u32,
                0u32,
                262394u32,
                508u32,
                509u32,
                510u32,
                131320u32,
                509u32,
                327745u32,
                31u32,
                511u32,
                18u32,
                30u32,
                262205u32,
                6u32,
                512u32,
                511u32,
                327745u32,
                31u32,
                513u32,
                19u32,
                30u32,
                262205u32,
                6u32,
                514u32,
                513u32,
                327855u32,
                28u32,
                515u32,
                512u32,
                514u32,
                131321u32,
                510u32,
                131320u32,
                510u32,
                458997u32,
                28u32,
                516u32,
                507u32,
                21u32,
                515u32,
                509u32,
                196855u32,
                518u32,
                0u32,
                262394u32,
                516u32,
                517u32,
                518u32,
                131320u32,
                517u32,
                65789u32,
                131320u32,
                518u32,
                327745u32,
                31u32,
                522u32,
                18u32,
                30u32,
                262205u32,
                6u32,
                523u32,
                522u32,
                327745u32,
                31u32,
                524u32,
                19u32,
                46u32,
                262205u32,
                6u32,
                525u32,
                524u32,
                327812u32,
                6u32,
                526u32,
                523u32,
                525u32,
                327745u32,
                31u32,
                527u32,
                18u32,
                46u32,
                262205u32,
                6u32,
                528u32,
                527u32,
                327808u32,
                6u32,
                529u32,
                526u32,
                528u32,
                393281u32,
                53u32,
                530u32,
                43u32,
                24u32,
                529u32,
                262205u32,
                29u32,
                531u32,
                530u32,
                196670u32,
                521u32,
                531u32,
                262205u32,
                29u32,
                532u32,
                521u32,
                327850u32,
                28u32,
                533u32,
                532u32,
                30u32,
                196855u32,
                535u32,
                0u32,
                262394u32,
                533u32,
                534u32,
                541u32,
                131320u32,
                534u32,
                262205u32,
                7u32,
                537u32,
                18u32,
                196670u32,
                536u32,
                537u32,
                262205u32,
                7u32,
                539u32,
                19u32,
                196670u32,
                538u32,
                539u32,
                393273u32,
                2u32,
                540u32,
                12u32,
                536u32,
                538u32,
                131321u32,
                535u32,
                131320u32,
                541u32,
                262205u32,
                29u32,
                542u32,
                521u32,
                327850u32,
                28u32,
                543u32,
                542u32,
                81u32,
                196855u32,
                545u32,
                0u32,
                262394u32,
                543u32,
                544u32,
                551u32,
                131320u32,
                544u32,
                262205u32,
                7u32,
                547u32,
                18u32,
                196670u32,
                546u32,
                547u32,
                262205u32,
                7u32,
                549u32,
                19u32,
                196670u32,
                548u32,
                549u32,
                393273u32,
                2u32,
                550u32,
                16u32,
                546u32,
                548u32,
                131321u32,
                545u32,
                131320u32,
                551u32,
                262205u32,
                29u32,
                552u32,
                521u32,
                327850u32,
                28u32,
                553u32,
                552u32,
                171u32,
                196855u32,
                555u32,
                0u32,
                262394u32,
                553u32,
                554u32,
                555u32,
                131320u32,
                554u32,
                131321u32,
                555u32,
                131320u32,
                555u32,
                131321u32,
                545u32,
                131320u32,
                545u32,
                131321u32,
                535u32,
                131320u32,
                535u32,
                65789u32,
                65592u32,
            ];
            unsafe {
                ::vulkano::shader::ShaderModule::new(
                    device,
                    ::vulkano::shader::ShaderModuleCreateInfo::new(&WORDS),
                )
            }
        }
        #[allow(non_camel_case_types, non_snake_case)]
        #[repr(C)]
        pub struct GridBuffer {
            pub grid: [u32],
        }
        #[allow(unsafe_code)]
        unsafe impl ::vulkano::buffer::BufferContents for GridBuffer {
            const LAYOUT: ::vulkano::buffer::BufferContentsLayout = {
                {
                    #[allow(unused)]
                    fn bound() {
                        fn assert_impl<T: ::vulkano::buffer::BufferContents + ?Sized>() {}
                        assert_impl::<u32>();
                    }
                }
                const fn extend_layout(
                    layout: ::std::alloc::Layout,
                    next: ::std::alloc::Layout,
                ) -> ::std::alloc::Layout {
                    let padded_size = if let Some(val) = layout.size().checked_add(next.align() - 1)
                    {
                        val & !(next.align() - 1)
                    } else {
                        ::core::panicking::panic("internal error: entered unreachable code")
                    };
                    let align = if layout.align() >= next.align() {
                        layout.align()
                    } else {
                        next.align()
                    };
                    if let Some(size) = padded_size.checked_add(next.size()) {
                        if let Ok(layout) = ::std::alloc::Layout::from_size_align(size, align) {
                            layout
                        } else {
                            ::core::panicking::panic("internal error: entered unreachable code")
                        }
                    } else {
                        ::core::panicking::panic("internal error: entered unreachable code")
                    }
                }
                if let Some(layout) =
                    ::vulkano::buffer::BufferContentsLayout::from_head_element_layout(
                        ::std::alloc::Layout::new::<()>(),
                        ::std::alloc::Layout::new::<u32>(),
                    )
                {
                    if let Some(layout) = layout.pad_to_alignment() {
                        layout
                    } else {
                        ::core::panicking::panic("internal error: entered unreachable code")
                    }
                } else {
                    {
                        ::core::panicking::panic_fmt(format_args!(
                            "zero-sized types are not valid buffer contents",
                        ));
                    }
                }
            };
            #[inline(always)]
            unsafe fn ptr_from_slice(slice: ::std::ptr::NonNull<[u8]>) -> *mut Self {
                #[repr(C)]
                union PtrRepr<T: ?Sized> {
                    components: PtrComponents,
                    ptr: *mut T,
                }
                #[repr(C)]
                struct PtrComponents {
                    data: *mut u8,
                    len: usize,
                }
                #[automatically_derived]
                impl ::core::clone::Clone for PtrComponents {
                    #[inline]
                    fn clone(&self) -> PtrComponents {
                        let _: ::core::clone::AssertParamIsClone<*mut u8>;
                        let _: ::core::clone::AssertParamIsClone<usize>;
                        *self
                    }
                }
                #[automatically_derived]
                impl ::core::marker::Copy for PtrComponents {}
                let data = <*mut [u8]>::cast::<u8>(slice.as_ptr());
                let head_size =
                    <Self as ::vulkano::buffer::BufferContents>::LAYOUT.head_size() as usize;
                let element_size = <Self as ::vulkano::buffer::BufferContents>::LAYOUT
                    .element_size()
                    .unwrap_or(1) as usize;
                if true {
                    if !(slice.len() >= head_size) {
                        ::core::panicking::panic("assertion failed: slice.len() >= head_size")
                    }
                }
                let tail_size = slice.len() - head_size;
                if true {
                    if !(tail_size % element_size == 0) {
                        ::core::panicking::panic("assertion failed: tail_size % element_size == 0")
                    }
                }
                let len = tail_size / element_size;
                let components = PtrComponents { data, len };
                PtrRepr { components }.ptr
            }
        }
        #[allow(non_camel_case_types, non_snake_case)]
        #[repr(C)]
        pub struct PushConstants {
            pub palette: [[f32; 4usize]; 4usize],
            pub simulate: u32,
        }
        #[allow(unsafe_code)]
        unsafe impl ::vulkano::buffer::BufferContents for PushConstants {
            const LAYOUT: ::vulkano::buffer::BufferContentsLayout = {
                {
                    #[allow(unused)]
                    fn bound() {
                        fn assert_impl<T: ::vulkano::buffer::BufferContents + ?Sized>() {}
                        assert_impl::<f32>();
                    }
                }
                {
                    #[allow(unused)]
                    fn bound() {
                        fn assert_impl<T: ::vulkano::buffer::BufferContents + ?Sized>() {}
                        assert_impl::<u32>();
                    }
                }
                const fn extend_layout(
                    layout: ::std::alloc::Layout,
                    next: ::std::alloc::Layout,
                ) -> ::std::alloc::Layout {
                    let padded_size = if let Some(val) = layout.size().checked_add(next.align() - 1)
                    {
                        val & !(next.align() - 1)
                    } else {
                        ::core::panicking::panic("internal error: entered unreachable code")
                    };
                    let align = if layout.align() >= next.align() {
                        layout.align()
                    } else {
                        next.align()
                    };
                    if let Some(size) = padded_size.checked_add(next.size()) {
                        if let Ok(layout) = ::std::alloc::Layout::from_size_align(size, align) {
                            layout
                        } else {
                            ::core::panicking::panic("internal error: entered unreachable code")
                        }
                    } else {
                        ::core::panicking::panic("internal error: entered unreachable code")
                    }
                }
                if let Some(layout) = <u32 as ::vulkano::buffer::BufferContents>::LAYOUT
                    .extend_from_layout(&extend_layout(
                        ::std::alloc::Layout::new::<()>(),
                        ::std::alloc::Layout::new::<[[f32; 4usize]; 4usize]>(),
                    ))
                {
                    if let Some(layout) = layout.pad_to_alignment() {
                        layout
                    } else {
                        ::core::panicking::panic("internal error: entered unreachable code")
                    }
                } else {
                    {
                        ::core::panicking::panic_fmt(format_args!(
                            "zero-sized types are not valid buffer contents",
                        ));
                    }
                }
            };
            #[inline(always)]
            unsafe fn ptr_from_slice(slice: ::std::ptr::NonNull<[u8]>) -> *mut Self {
                #[repr(C)]
                union PtrRepr<T: ?Sized> {
                    components: PtrComponents,
                    ptr: *mut T,
                }
                #[repr(C)]
                struct PtrComponents {
                    data: *mut u8,
                    len: usize,
                }
                #[automatically_derived]
                impl ::core::clone::Clone for PtrComponents {
                    #[inline]
                    fn clone(&self) -> PtrComponents {
                        let _: ::core::clone::AssertParamIsClone<*mut u8>;
                        let _: ::core::clone::AssertParamIsClone<usize>;
                        *self
                    }
                }
                #[automatically_derived]
                impl ::core::marker::Copy for PtrComponents {}
                let data = <*mut [u8]>::cast::<u8>(slice.as_ptr());
                let head_size =
                    <Self as ::vulkano::buffer::BufferContents>::LAYOUT.head_size() as usize;
                let element_size = <Self as ::vulkano::buffer::BufferContents>::LAYOUT
                    .element_size()
                    .unwrap_or(1) as usize;
                if true {
                    if !(slice.len() >= head_size) {
                        ::core::panicking::panic("assertion failed: slice.len() >= head_size")
                    }
                }
                let tail_size = slice.len() - head_size;
                if true {
                    if !(tail_size % element_size == 0) {
                        ::core::panicking::panic("assertion failed: tail_size % element_size == 0")
                    }
                }
                let len = tail_size / element_size;
                let components = PtrComponents { data, len };
                PtrRepr { components }.ptr
            }
        }
        #[automatically_derived]
        #[allow(non_camel_case_types, non_snake_case)]
        impl ::core::clone::Clone for PushConstants {
            #[inline]
            fn clone(&self) -> PushConstants {
                let _: ::core::clone::AssertParamIsClone<[[f32; 4usize]; 4usize]>;
                let _: ::core::clone::AssertParamIsClone<u32>;
                *self
            }
        }
        #[automatically_derived]
        #[allow(non_camel_case_types, non_snake_case)]
        impl ::core::marker::Copy for PushConstants {}
    }
    pub enum CellType {
        Empty,
        Sand,
        Wall,
        Water,
    }
    #[automatically_derived]
    impl ::core::clone::Clone for CellType {
        #[inline]
        fn clone(&self) -> CellType {
            *self
        }
    }
    #[automatically_derived]
    impl ::core::marker::Copy for CellType {}
    #[automatically_derived]
    impl ::core::marker::StructuralPartialEq for CellType {}
    #[automatically_derived]
    impl ::core::cmp::PartialEq for CellType {
        #[inline]
        fn eq(&self, other: &CellType) -> bool {
            let __self_discr = ::core::intrinsics::discriminant_value(self);
            let __arg1_discr = ::core::intrinsics::discriminant_value(other);
            __self_discr == __arg1_discr
        }
    }
    #[automatically_derived]
    impl ::core::cmp::Eq for CellType {
        #[inline]
        #[doc(hidden)]
        #[coverage(off)]
        fn assert_receiver_is_total_eq(&self) -> () {}
    }
    ///An iterator over the variants of [CellType]
    #[allow(missing_copy_implementations)]
    pub struct CellTypeIter {
        idx: usize,
        back_idx: usize,
        marker: ::core::marker::PhantomData<()>,
    }
    impl ::core::fmt::Debug for CellTypeIter {
        fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
            f.debug_struct("CellTypeIter")
                .field("len", &self.len())
                .finish()
        }
    }
    impl CellTypeIter {
        fn get(&self, idx: usize) -> ::core::option::Option<CellType> {
            match idx {
                0usize => ::core::option::Option::Some(CellType::Empty),
                1usize => ::core::option::Option::Some(CellType::Sand),
                2usize => ::core::option::Option::Some(CellType::Wall),
                3usize => ::core::option::Option::Some(CellType::Water),
                _ => ::core::option::Option::None,
            }
        }
    }
    impl ::strum::IntoEnumIterator for CellType {
        type Iterator = CellTypeIter;
        fn iter() -> CellTypeIter {
            CellTypeIter {
                idx: 0,
                back_idx: 0,
                marker: ::core::marker::PhantomData,
            }
        }
    }
    impl Iterator for CellTypeIter {
        type Item = CellType;
        fn next(&mut self) -> ::core::option::Option<<Self as Iterator>::Item> {
            self.nth(0)
        }
        fn size_hint(&self) -> (usize, ::core::option::Option<usize>) {
            let t = if self.idx + self.back_idx >= 4usize {
                0
            } else {
                4usize - self.idx - self.back_idx
            };
            (t, Some(t))
        }
        fn nth(&mut self, n: usize) -> ::core::option::Option<<Self as Iterator>::Item> {
            let idx = self.idx + n + 1;
            if idx + self.back_idx > 4usize {
                self.idx = 4usize;
                ::core::option::Option::None
            } else {
                self.idx = idx;
                CellTypeIter::get(self, idx - 1)
            }
        }
    }
    impl ExactSizeIterator for CellTypeIter {
        fn len(&self) -> usize {
            self.size_hint().0
        }
    }
    impl DoubleEndedIterator for CellTypeIter {
        fn next_back(&mut self) -> ::core::option::Option<<Self as Iterator>::Item> {
            let back_idx = self.back_idx + 1;
            if self.idx + back_idx > 4usize {
                self.back_idx = 4usize;
                ::core::option::Option::None
            } else {
                self.back_idx = back_idx;
                CellTypeIter::get(self, 4usize - self.back_idx)
            }
        }
    }
    impl ::core::iter::FusedIterator for CellTypeIter {}
    impl Clone for CellTypeIter {
        fn clone(&self) -> CellTypeIter {
            CellTypeIter {
                idx: self.idx,
                back_idx: self.back_idx,
                marker: self.marker.clone(),
            }
        }
    }
    impl ::core::fmt::Display for CellType {
        fn fmt(
            &self,
            f: &mut ::core::fmt::Formatter,
        ) -> ::core::result::Result<(), ::core::fmt::Error> {
            match *self {
                CellType::Empty => ::core::fmt::Display::fmt("Empty", f),
                CellType::Sand => ::core::fmt::Display::fmt("Sand", f),
                CellType::Wall => ::core::fmt::Display::fmt("Wall", f),
                CellType::Water => ::core::fmt::Display::fmt("Water", f),
            }
        }
    }
    pub enum BrushType {
        CircleFull,
        CircleHollow,
        Cube,
    }
    #[automatically_derived]
    impl ::core::clone::Clone for BrushType {
        #[inline]
        fn clone(&self) -> BrushType {
            *self
        }
    }
    #[automatically_derived]
    impl ::core::marker::Copy for BrushType {}
    #[automatically_derived]
    impl ::core::marker::StructuralPartialEq for BrushType {}
    #[automatically_derived]
    impl ::core::cmp::PartialEq for BrushType {
        #[inline]
        fn eq(&self, other: &BrushType) -> bool {
            let __self_discr = ::core::intrinsics::discriminant_value(self);
            let __arg1_discr = ::core::intrinsics::discriminant_value(other);
            __self_discr == __arg1_discr
        }
    }
    #[automatically_derived]
    impl ::core::cmp::Eq for BrushType {
        #[inline]
        #[doc(hidden)]
        #[coverage(off)]
        fn assert_receiver_is_total_eq(&self) -> () {}
    }
    ///An iterator over the variants of [BrushType]
    #[allow(missing_copy_implementations)]
    pub struct BrushTypeIter {
        idx: usize,
        back_idx: usize,
        marker: ::core::marker::PhantomData<()>,
    }
    impl ::core::fmt::Debug for BrushTypeIter {
        fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
            f.debug_struct("BrushTypeIter")
                .field("len", &self.len())
                .finish()
        }
    }
    impl BrushTypeIter {
        fn get(&self, idx: usize) -> ::core::option::Option<BrushType> {
            match idx {
                0usize => ::core::option::Option::Some(BrushType::CircleFull),
                1usize => ::core::option::Option::Some(BrushType::CircleHollow),
                2usize => ::core::option::Option::Some(BrushType::Cube),
                _ => ::core::option::Option::None,
            }
        }
    }
    impl ::strum::IntoEnumIterator for BrushType {
        type Iterator = BrushTypeIter;
        fn iter() -> BrushTypeIter {
            BrushTypeIter {
                idx: 0,
                back_idx: 0,
                marker: ::core::marker::PhantomData,
            }
        }
    }
    impl Iterator for BrushTypeIter {
        type Item = BrushType;
        fn next(&mut self) -> ::core::option::Option<<Self as Iterator>::Item> {
            self.nth(0)
        }
        fn size_hint(&self) -> (usize, ::core::option::Option<usize>) {
            let t = if self.idx + self.back_idx >= 3usize {
                0
            } else {
                3usize - self.idx - self.back_idx
            };
            (t, Some(t))
        }
        fn nth(&mut self, n: usize) -> ::core::option::Option<<Self as Iterator>::Item> {
            let idx = self.idx + n + 1;
            if idx + self.back_idx > 3usize {
                self.idx = 3usize;
                ::core::option::Option::None
            } else {
                self.idx = idx;
                BrushTypeIter::get(self, idx - 1)
            }
        }
    }
    impl ExactSizeIterator for BrushTypeIter {
        fn len(&self) -> usize {
            self.size_hint().0
        }
    }
    impl DoubleEndedIterator for BrushTypeIter {
        fn next_back(&mut self) -> ::core::option::Option<<Self as Iterator>::Item> {
            let back_idx = self.back_idx + 1;
            if self.idx + back_idx > 3usize {
                self.back_idx = 3usize;
                ::core::option::Option::None
            } else {
                self.back_idx = back_idx;
                BrushTypeIter::get(self, 3usize - self.back_idx)
            }
        }
    }
    impl ::core::iter::FusedIterator for BrushTypeIter {}
    impl Clone for BrushTypeIter {
        fn clone(&self) -> BrushTypeIter {
            BrushTypeIter {
                idx: self.idx,
                back_idx: self.back_idx,
                marker: self.marker.clone(),
            }
        }
    }
    impl ::core::fmt::Display for BrushType {
        fn fmt(
            &self,
            f: &mut ::core::fmt::Formatter,
        ) -> ::core::result::Result<(), ::core::fmt::Error> {
            match *self {
                BrushType::CircleFull => ::core::fmt::Display::fmt("CircleFull", f),
                BrushType::CircleHollow => ::core::fmt::Display::fmt("CircleHollow", f),
                BrushType::Cube => ::core::fmt::Display::fmt("Cube", f),
            }
        }
    }
}
pub mod gui {
    use crate::{
        compute_sand::{BrushType, CellType, SandComputePipeline},
        state::SimClock,
    };
    use egui_winit_vulkano::{egui, Gui, GuiConfig};
    use log::info;
    use std::{fmt::Display, str::FromStr, sync::Arc};
    use strum::IntoEnumIterator;
    use vulkano::{
        device::Queue, format::Format, image::view::ImageView, swapchain::Surface, sync::GpuFuture,
    };
    use winit::{event::WindowEvent, event_loop::ActiveEventLoop};
    pub struct GameGui {
        gui: Gui,
    }
    impl GameGui {
        pub fn new(
            event_loop: &ActiveEventLoop,
            surface: Arc<Surface>,
            gfx_queue: Arc<Queue>,
            output_format: Format,
        ) -> Self {
            let gui = Gui::new(
                event_loop,
                surface,
                gfx_queue,
                output_format,
                GuiConfig {
                    allow_srgb_render_target: true,
                    is_overlay: true,
                    samples: vulkano::image::SampleCount::Sample1,
                },
            );
            GameGui { gui }
        }
        pub fn event(&mut self, event: &WindowEvent) {
            self.gui.update(event);
        }
        pub fn draw_gui(
            &mut self,
            sim_clock: &mut SimClock,
            compute: &mut SandComputePipeline,
            is_hovered: &mut bool,
            selected_cell_type: &mut CellType,
            size: [u32; 2],
            background_color: &mut [f32; 4],
        ) {
            let (simulate_ui_togle, cur_sim, &mut (mut sim_rate)) = sim_clock.ui_togles();
            self.gui.immediate_ui(|gui| {
                let ctx = gui.context();
                let mut pointer_on_debug_window = false;
                egui::Window::new("Grid setup").show(&ctx, |ui| {
                    ui.checkbox(simulate_ui_togle, "Simulate");
                    let sim_speed_slider =
                        ui.add(egui::Slider::new(cur_sim, 0..=100).text("Sim speed"));
                    if sim_speed_slider.changed() {}
                    if ui
                        .add(
                            egui::Slider::new(&mut compute.scale_factor, 0..=100)
                                .text("Grid scale factor"),
                        )
                        .changed()
                    {
                        compute.resize(size)
                    }
                    if ui.button("New Random Grid").clicked() {
                        compute.new_rand_grid()
                    }
                    ui.label(::alloc::__export::must_use({
                        let res = ::alloc::fmt::format(format_args!("sim_rate: {0}", sim_rate));
                        res
                    }));
                    pointer_on_debug_window = ui.ui_contains_pointer();
                });
                let mut pointer_on_selector_window = false;
                egui::Window::new("Cell Type selector").show(&ctx, |ui| {
                    for (i, cell_type) in CellType::iter().enumerate() {
                        if i != 0 {
                            ui.radio_value(selected_cell_type, cell_type, cell_type.to_string());
                        }
                    }
                    pointer_on_selector_window = ui.ui_contains_pointer();
                });
                let mut pointer_on_color_window = false;
                egui::Window::new("Palette editor").show(&ctx, |ui| {
                    ui.label("Background color:");
                    ui.color_edit_button_rgba_premultiplied(background_color);
                    ui.label("Cells pallete:");
                    for color in compute.pallete.iter_mut() {
                        ui.color_edit_button_rgba_premultiplied(color);
                    }
                    pointer_on_color_window = ui.ui_contains_pointer();
                });
                let mut pointer_on_brush_window = false;
                egui::Window::new("Brush editor").show(&ctx, |ui| {
                    ui.add(egui::Slider::new(&mut compute.brush_size, 0..=100).text("Brush size"));
                    for brush_type in BrushType::iter() {
                        ui.radio_value(
                            &mut compute.selected_brush,
                            brush_type,
                            brush_type.to_string(),
                        );
                    }
                    pointer_on_brush_window = ui.ui_contains_pointer();
                });
                if pointer_on_debug_window
                    || pointer_on_selector_window
                    || pointer_on_color_window
                    || pointer_on_brush_window
                {
                    *is_hovered = true
                } else {
                    *is_hovered = false
                }
            });
        }
        pub fn draw_on_image<F>(
            &mut self,
            before_future: F,
            final_image: Arc<ImageView>,
        ) -> Box<dyn GpuFuture>
        where
            F: GpuFuture + 'static,
        {
            self.gui.draw_on_image(before_future, final_image)
        }
    }
    fn integer_edit_field<T>(ui: &mut egui::Ui, value: &mut T) -> egui::Response
    where
        T: Display,
        T: FromStr,
    {
        let mut tmp_value = ::alloc::__export::must_use({
            let res = ::alloc::fmt::format(format_args!("{0}", value));
            res
        });
        let res = ui.text_edit_singleline(&mut tmp_value);
        if let Ok(result) = tmp_value.parse() {
            *value = result;
        }
        res
    }
}
pub mod pixels_draw {
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
        image::{
            sampler::{Filter, Sampler, SamplerAddressMode, SamplerCreateInfo, SamplerMipmapMode},
            view::ImageView,
        },
        memory::allocator::{AllocationCreateInfo, MemoryTypeFilter, StandardMemoryAllocator},
        pipeline::{
            graphics::{
                color_blend::{
                    AttachmentBlend, BlendFactor, BlendOp, ColorBlendAttachmentState,
                    ColorBlendState,
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
    #[repr(C)]
    pub struct TexturedVertex {
        #[format(R32G32_SFLOAT)]
        pub position: [f32; 2],
        #[format(R32G32_SFLOAT)]
        pub tex_coords: [f32; 2],
    }
    #[allow(unsafe_code)]
    unsafe impl ::vulkano::buffer::BufferContents for TexturedVertex {
        const LAYOUT: ::vulkano::buffer::BufferContentsLayout = {
            {
                #[allow(unused)]
                fn bound() {
                    fn assert_impl<T: ::vulkano::buffer::BufferContents + ?Sized>() {}
                    assert_impl::<f32>();
                }
            }
            {
                #[allow(unused)]
                fn bound() {
                    fn assert_impl<T: ::vulkano::buffer::BufferContents + ?Sized>() {}
                    assert_impl::<f32>();
                }
            }
            const fn extend_layout(
                layout: ::std::alloc::Layout,
                next: ::std::alloc::Layout,
            ) -> ::std::alloc::Layout {
                let padded_size = if let Some(val) = layout.size().checked_add(next.align() - 1) {
                    val & !(next.align() - 1)
                } else {
                    ::core::panicking::panic("internal error: entered unreachable code")
                };
                let align = if layout.align() >= next.align() {
                    layout.align()
                } else {
                    next.align()
                };
                if let Some(size) = padded_size.checked_add(next.size()) {
                    if let Ok(layout) = ::std::alloc::Layout::from_size_align(size, align) {
                        layout
                    } else {
                        ::core::panicking::panic("internal error: entered unreachable code")
                    }
                } else {
                    ::core::panicking::panic("internal error: entered unreachable code")
                }
            }
            if let Some(layout) = ::vulkano::buffer::BufferContentsLayout::from_sized(
                ::std::alloc::Layout::new::<Self>(),
            ) {
                if let Some(layout) = layout.pad_to_alignment() {
                    layout
                } else {
                    ::core::panicking::panic("internal error: entered unreachable code")
                }
            } else {
                {
                    ::core::panicking::panic_fmt(format_args!(
                        "zero-sized types are not valid buffer contents"
                    ));
                }
            }
        };
        #[inline(always)]
        unsafe fn ptr_from_slice(slice: ::std::ptr::NonNull<[u8]>) -> *mut Self {
            #[repr(C)]
            union PtrRepr<T: ?Sized> {
                components: PtrComponents,
                ptr: *mut T,
            }
            #[repr(C)]
            struct PtrComponents {
                data: *mut u8,
                len: usize,
            }
            #[automatically_derived]
            impl ::core::clone::Clone for PtrComponents {
                #[inline]
                fn clone(&self) -> PtrComponents {
                    let _: ::core::clone::AssertParamIsClone<*mut u8>;
                    let _: ::core::clone::AssertParamIsClone<usize>;
                    *self
                }
            }
            #[automatically_derived]
            impl ::core::marker::Copy for PtrComponents {}
            let data = <*mut [u8]>::cast::<u8>(slice.as_ptr());
            let head_size =
                <Self as ::vulkano::buffer::BufferContents>::LAYOUT.head_size() as usize;
            let element_size = <Self as ::vulkano::buffer::BufferContents>::LAYOUT
                .element_size()
                .unwrap_or(1) as usize;
            if true {
                if !(slice.len() >= head_size) {
                    ::core::panicking::panic("assertion failed: slice.len() >= head_size")
                }
            }
            let tail_size = slice.len() - head_size;
            if true {
                if !(tail_size % element_size == 0) {
                    ::core::panicking::panic("assertion failed: tail_size % element_size == 0")
                }
            }
            let len = tail_size / element_size;
            let components = PtrComponents { data, len };
            PtrRepr { components }.ptr
        }
    }
    #[allow(unsafe_code)]
    unsafe impl ::vulkano::pipeline::graphics::vertex_input::Vertex for TexturedVertex {
        #[inline(always)]
        fn per_vertex() -> ::vulkano::pipeline::graphics::vertex_input::VertexBufferDescription {
            let mut offset = 0;
            let mut members = ::std::collections::HashMap::default();
            {
                let field_align = ::std::mem::align_of::<[f32; 2]>();
                offset = (offset + field_align - 1) & !(field_align - 1);
                let field_size = ::std::mem::size_of::<[f32; 2]>();
                let format = ::vulkano::format::Format::R32G32_SFLOAT;
                let format_size = format.block_size() as usize;
                let num_elements = field_size / format_size;
                let remainder = field_size % format_size;
                if !(remainder == 0) {
                    {
                        ::core::panicking::panic_fmt(format_args!(
                            "struct field `{0}` size does not fit multiple of format size",
                            "position",
                        ));
                    }
                }
                members.insert(
                    "position".to_string(),
                    ::vulkano::pipeline::graphics::vertex_input::VertexMemberInfo {
                        offset: offset.try_into().unwrap(),
                        format,
                        num_elements: num_elements.try_into().unwrap(),
                    },
                );
                offset += field_size;
            }
            {
                let field_align = ::std::mem::align_of::<[f32; 2]>();
                offset = (offset + field_align - 1) & !(field_align - 1);
                let field_size = ::std::mem::size_of::<[f32; 2]>();
                let format = ::vulkano::format::Format::R32G32_SFLOAT;
                let format_size = format.block_size() as usize;
                let num_elements = field_size / format_size;
                let remainder = field_size % format_size;
                if !(remainder == 0) {
                    {
                        ::core::panicking::panic_fmt(format_args!(
                            "struct field `{0}` size does not fit multiple of format size",
                            "tex_coords",
                        ));
                    }
                }
                members.insert(
                    "tex_coords".to_string(),
                    ::vulkano::pipeline::graphics::vertex_input::VertexMemberInfo {
                        offset: offset.try_into().unwrap(),
                        format,
                        num_elements: num_elements.try_into().unwrap(),
                    },
                );
                offset += field_size;
            }
            ::vulkano::pipeline::graphics::vertex_input::VertexBufferDescription {
                members,
                stride: ::std::mem::size_of::<TexturedVertex>() as u32,
                input_rate: ::vulkano::pipeline::graphics::vertex_input::VertexInputRate::Vertex,
            }
        }
        #[inline(always)]
        fn per_instance() -> ::vulkano::pipeline::graphics::vertex_input::VertexBufferDescription {
            Self::per_vertex().per_instance()
        }
        #[inline(always)]
        fn per_instance_with_divisor(
            divisor: u32,
        ) -> ::vulkano::pipeline::graphics::vertex_input::VertexBufferDescription {
            Self::per_vertex().per_instance_with_divisor(divisor)
        }
    }
    pub fn textured_quad(width: f32, height: f32) -> (Vec<TexturedVertex>, Vec<u32>) {
        (
            <[_]>::into_vec(
                #[rustc_box]
                ::alloc::boxed::Box::new([
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
                ]),
            ),
            <[_]>::into_vec(
                #[rustc_box]
                ::alloc::boxed::Box::new([0, 2, 1, 0, 3, 2]),
            ),
        )
    }
    /// A subpass pipeline that fills a quad over the frame.
    pub struct PixelsDrawPipeline {
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
    impl PixelsDrawPipeline {
        pub fn new(app: &Renderer, subpass: Subpass) -> PixelsDrawPipeline {
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
                                    src_color_blend_factor: BlendFactor::SrcAlpha,
                                    dst_color_blend_factor: BlendFactor::OneMinusSrcAlpha,
                                    color_blend_op: BlendOp::Add,
                                    src_alpha_blend_factor: BlendFactor::One,
                                    dst_alpha_blend_factor: BlendFactor::Zero,
                                    alpha_blend_op: BlendOp::Add,
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
            PixelsDrawPipeline {
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
            let buffer = Buffer::from_data(
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
                background_color,
            )
            .unwrap();
            PersistentDescriptorSet::new(
                &self.descriptor_set_allocator,
                layout.clone(),
                [
                    WriteDescriptorSet::buffer(0, buffer),
                    WriteDescriptorSet::sampler(1, sampler),
                    WriteDescriptorSet::image_view(2, image),
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
            let desc_set = self.create_image_sampler_nearest(image, background_color);
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
    mod vs {
        /// Loads the shader as a `ShaderModule`.
        #[allow(unsafe_code)]
        #[inline]
        pub fn load(
            device: ::std::sync::Arc<::vulkano::device::Device>,
        ) -> ::std::result::Result<
            ::std::sync::Arc<::vulkano::shader::ShaderModule>,
            ::vulkano::Validated<::vulkano::VulkanError>,
        > {
            let _bytes = (b"#version 450\nlayout(location=0) in vec2 position;\nlayout(location=1) in vec2 tex_coords;\n\nlayout(location = 0) out vec2 f_tex_coords;\n\nvoid main() {\n    gl_Position =  vec4(position, 0.0, 1.0);\n    f_tex_coords = tex_coords;\n}\n");
            static WORDS: &[u32] = &[
                119734787u32,
                65536u32,
                851979u32,
                31u32,
                0u32,
                131089u32,
                1u32,
                393227u32,
                1u32,
                1280527431u32,
                1685353262u32,
                808793134u32,
                0u32,
                196622u32,
                0u32,
                1u32,
                589839u32,
                0u32,
                4u32,
                1852399981u32,
                0u32,
                13u32,
                18u32,
                28u32,
                29u32,
                196611u32,
                2u32,
                450u32,
                655364u32,
                1197427783u32,
                1279741775u32,
                1885560645u32,
                1953718128u32,
                1600482425u32,
                1701734764u32,
                1919509599u32,
                1769235301u32,
                25974u32,
                524292u32,
                1197427783u32,
                1279741775u32,
                1852399429u32,
                1685417059u32,
                1768185701u32,
                1952671090u32,
                6649449u32,
                262149u32,
                4u32,
                1852399981u32,
                0u32,
                393221u32,
                11u32,
                1348430951u32,
                1700164197u32,
                2019914866u32,
                0u32,
                393222u32,
                11u32,
                0u32,
                1348430951u32,
                1953067887u32,
                7237481u32,
                458758u32,
                11u32,
                1u32,
                1348430951u32,
                1953393007u32,
                1702521171u32,
                0u32,
                458758u32,
                11u32,
                2u32,
                1130327143u32,
                1148217708u32,
                1635021673u32,
                6644590u32,
                458758u32,
                11u32,
                3u32,
                1130327143u32,
                1147956341u32,
                1635021673u32,
                6644590u32,
                196613u32,
                13u32,
                0u32,
                327685u32,
                18u32,
                1769172848u32,
                1852795252u32,
                0u32,
                393221u32,
                28u32,
                1702125414u32,
                1868783480u32,
                1935962735u32,
                0u32,
                327685u32,
                29u32,
                1601725812u32,
                1919905635u32,
                29540u32,
                327752u32,
                11u32,
                0u32,
                11u32,
                0u32,
                327752u32,
                11u32,
                1u32,
                11u32,
                1u32,
                327752u32,
                11u32,
                2u32,
                11u32,
                3u32,
                327752u32,
                11u32,
                3u32,
                11u32,
                4u32,
                196679u32,
                11u32,
                2u32,
                262215u32,
                18u32,
                30u32,
                0u32,
                262215u32,
                28u32,
                30u32,
                0u32,
                262215u32,
                29u32,
                30u32,
                1u32,
                131091u32,
                2u32,
                196641u32,
                3u32,
                2u32,
                196630u32,
                6u32,
                32u32,
                262167u32,
                7u32,
                6u32,
                4u32,
                262165u32,
                8u32,
                32u32,
                0u32,
                262187u32,
                8u32,
                9u32,
                1u32,
                262172u32,
                10u32,
                6u32,
                9u32,
                393246u32,
                11u32,
                7u32,
                6u32,
                10u32,
                10u32,
                262176u32,
                12u32,
                3u32,
                11u32,
                262203u32,
                12u32,
                13u32,
                3u32,
                262165u32,
                14u32,
                32u32,
                1u32,
                262187u32,
                14u32,
                15u32,
                0u32,
                262167u32,
                16u32,
                6u32,
                2u32,
                262176u32,
                17u32,
                1u32,
                16u32,
                262203u32,
                17u32,
                18u32,
                1u32,
                262187u32,
                6u32,
                20u32,
                0u32,
                262187u32,
                6u32,
                21u32,
                1065353216u32,
                262176u32,
                25u32,
                3u32,
                7u32,
                262176u32,
                27u32,
                3u32,
                16u32,
                262203u32,
                27u32,
                28u32,
                3u32,
                262203u32,
                17u32,
                29u32,
                1u32,
                327734u32,
                2u32,
                4u32,
                0u32,
                3u32,
                131320u32,
                5u32,
                262205u32,
                16u32,
                19u32,
                18u32,
                327761u32,
                6u32,
                22u32,
                19u32,
                0u32,
                327761u32,
                6u32,
                23u32,
                19u32,
                1u32,
                458832u32,
                7u32,
                24u32,
                22u32,
                23u32,
                20u32,
                21u32,
                327745u32,
                25u32,
                26u32,
                13u32,
                15u32,
                196670u32,
                26u32,
                24u32,
                262205u32,
                16u32,
                30u32,
                29u32,
                196670u32,
                28u32,
                30u32,
                65789u32,
                65592u32,
            ];
            unsafe {
                ::vulkano::shader::ShaderModule::new(
                    device,
                    ::vulkano::shader::ShaderModuleCreateInfo::new(&WORDS),
                )
            }
        }
    }
    mod fs {
        /// Loads the shader as a `ShaderModule`.
        #[allow(unsafe_code)]
        #[inline]
        pub fn load(
            device: ::std::sync::Arc<::vulkano::device::Device>,
        ) -> ::std::result::Result<
            ::std::sync::Arc<::vulkano::shader::ShaderModule>,
            ::vulkano::Validated<::vulkano::VulkanError>,
        > {
            let _bytes = (b"#version 450\nlayout(location = 0) in vec2 v_tex_coords;\nlayout(location = 0) out vec4 f_color;\n\n// Uniform Block Declaration (without the sampler)\nlayout(set = 0, binding = 0) uniform UBO {\n    vec4 background_color;\n};\n\n// Sampler Declaration with its own binding\nlayout(set = 0, binding = 1) uniform sampler s;\n\nlayout(set = 0, binding = 2) uniform texture2D tex;\n\nvoid main() {\n    vec4 color = texture(sampler2D(tex, s), v_tex_coords);\n    f_color = mix(background_color, color, color.a);\n}\n");
            static WORDS: &[u32] = &[
                119734787u32,
                65536u32,
                851979u32,
                43u32,
                0u32,
                131089u32,
                1u32,
                393227u32,
                1u32,
                1280527431u32,
                1685353262u32,
                808793134u32,
                0u32,
                196622u32,
                0u32,
                1u32,
                458767u32,
                4u32,
                4u32,
                1852399981u32,
                0u32,
                22u32,
                26u32,
                196624u32,
                4u32,
                7u32,
                196611u32,
                2u32,
                450u32,
                655364u32,
                1197427783u32,
                1279741775u32,
                1885560645u32,
                1953718128u32,
                1600482425u32,
                1701734764u32,
                1919509599u32,
                1769235301u32,
                25974u32,
                524292u32,
                1197427783u32,
                1279741775u32,
                1852399429u32,
                1685417059u32,
                1768185701u32,
                1952671090u32,
                6649449u32,
                262149u32,
                4u32,
                1852399981u32,
                0u32,
                262149u32,
                9u32,
                1869377379u32,
                114u32,
                196613u32,
                12u32,
                7890292u32,
                196613u32,
                16u32,
                115u32,
                393221u32,
                22u32,
                1702125430u32,
                1868783480u32,
                1935962735u32,
                0u32,
                262149u32,
                26u32,
                1868783462u32,
                7499628u32,
                196613u32,
                27u32,
                5194325u32,
                524294u32,
                27u32,
                0u32,
                1801675106u32,
                1970238055u32,
                1667196014u32,
                1919904879u32,
                0u32,
                196613u32,
                29u32,
                0u32,
                262215u32,
                12u32,
                34u32,
                0u32,
                262215u32,
                12u32,
                33u32,
                2u32,
                262215u32,
                16u32,
                34u32,
                0u32,
                262215u32,
                16u32,
                33u32,
                1u32,
                262215u32,
                22u32,
                30u32,
                0u32,
                262215u32,
                26u32,
                30u32,
                0u32,
                327752u32,
                27u32,
                0u32,
                35u32,
                0u32,
                196679u32,
                27u32,
                2u32,
                262215u32,
                29u32,
                34u32,
                0u32,
                262215u32,
                29u32,
                33u32,
                0u32,
                131091u32,
                2u32,
                196641u32,
                3u32,
                2u32,
                196630u32,
                6u32,
                32u32,
                262167u32,
                7u32,
                6u32,
                4u32,
                262176u32,
                8u32,
                7u32,
                7u32,
                589849u32,
                10u32,
                6u32,
                1u32,
                0u32,
                0u32,
                0u32,
                1u32,
                0u32,
                262176u32,
                11u32,
                0u32,
                10u32,
                262203u32,
                11u32,
                12u32,
                0u32,
                131098u32,
                14u32,
                262176u32,
                15u32,
                0u32,
                14u32,
                262203u32,
                15u32,
                16u32,
                0u32,
                196635u32,
                18u32,
                10u32,
                262167u32,
                20u32,
                6u32,
                2u32,
                262176u32,
                21u32,
                1u32,
                20u32,
                262203u32,
                21u32,
                22u32,
                1u32,
                262176u32,
                25u32,
                3u32,
                7u32,
                262203u32,
                25u32,
                26u32,
                3u32,
                196638u32,
                27u32,
                7u32,
                262176u32,
                28u32,
                2u32,
                27u32,
                262203u32,
                28u32,
                29u32,
                2u32,
                262165u32,
                30u32,
                32u32,
                1u32,
                262187u32,
                30u32,
                31u32,
                0u32,
                262176u32,
                32u32,
                2u32,
                7u32,
                262165u32,
                36u32,
                32u32,
                0u32,
                262187u32,
                36u32,
                37u32,
                3u32,
                262176u32,
                38u32,
                7u32,
                6u32,
                327734u32,
                2u32,
                4u32,
                0u32,
                3u32,
                131320u32,
                5u32,
                262203u32,
                8u32,
                9u32,
                7u32,
                262205u32,
                10u32,
                13u32,
                12u32,
                262205u32,
                14u32,
                17u32,
                16u32,
                327766u32,
                18u32,
                19u32,
                13u32,
                17u32,
                262205u32,
                20u32,
                23u32,
                22u32,
                327767u32,
                7u32,
                24u32,
                19u32,
                23u32,
                196670u32,
                9u32,
                24u32,
                327745u32,
                32u32,
                33u32,
                29u32,
                31u32,
                262205u32,
                7u32,
                34u32,
                33u32,
                262205u32,
                7u32,
                35u32,
                9u32,
                327745u32,
                38u32,
                39u32,
                9u32,
                37u32,
                262205u32,
                6u32,
                40u32,
                39u32,
                458832u32,
                7u32,
                41u32,
                40u32,
                40u32,
                40u32,
                40u32,
                524300u32,
                7u32,
                42u32,
                1u32,
                46u32,
                34u32,
                35u32,
                41u32,
                196670u32,
                26u32,
                42u32,
                65789u32,
                65592u32,
            ];
            unsafe {
                ::vulkano::shader::ShaderModule::new(
                    device,
                    ::vulkano::shader::ShaderModuleCreateInfo::new(&WORDS),
                )
            }
        }
        #[allow(non_camel_case_types, non_snake_case)]
        #[repr(C)]
        pub struct UBO {
            pub background_color: [f32; 4usize],
        }
        #[allow(unsafe_code)]
        unsafe impl ::vulkano::buffer::BufferContents for UBO {
            const LAYOUT: ::vulkano::buffer::BufferContentsLayout = {
                {
                    #[allow(unused)]
                    fn bound() {
                        fn assert_impl<T: ::vulkano::buffer::BufferContents + ?Sized>() {}
                        assert_impl::<f32>();
                    }
                }
                const fn extend_layout(
                    layout: ::std::alloc::Layout,
                    next: ::std::alloc::Layout,
                ) -> ::std::alloc::Layout {
                    let padded_size = if let Some(val) = layout.size().checked_add(next.align() - 1)
                    {
                        val & !(next.align() - 1)
                    } else {
                        ::core::panicking::panic("internal error: entered unreachable code")
                    };
                    let align = if layout.align() >= next.align() {
                        layout.align()
                    } else {
                        next.align()
                    };
                    if let Some(size) = padded_size.checked_add(next.size()) {
                        if let Ok(layout) = ::std::alloc::Layout::from_size_align(size, align) {
                            layout
                        } else {
                            ::core::panicking::panic("internal error: entered unreachable code")
                        }
                    } else {
                        ::core::panicking::panic("internal error: entered unreachable code")
                    }
                }
                if let Some(layout) = ::vulkano::buffer::BufferContentsLayout::from_sized(
                    ::std::alloc::Layout::new::<Self>(),
                ) {
                    if let Some(layout) = layout.pad_to_alignment() {
                        layout
                    } else {
                        ::core::panicking::panic("internal error: entered unreachable code")
                    }
                } else {
                    {
                        ::core::panicking::panic_fmt(format_args!(
                            "zero-sized types are not valid buffer contents",
                        ));
                    }
                }
            };
            #[inline(always)]
            unsafe fn ptr_from_slice(slice: ::std::ptr::NonNull<[u8]>) -> *mut Self {
                #[repr(C)]
                union PtrRepr<T: ?Sized> {
                    components: PtrComponents,
                    ptr: *mut T,
                }
                #[repr(C)]
                struct PtrComponents {
                    data: *mut u8,
                    len: usize,
                }
                #[automatically_derived]
                impl ::core::clone::Clone for PtrComponents {
                    #[inline]
                    fn clone(&self) -> PtrComponents {
                        let _: ::core::clone::AssertParamIsClone<*mut u8>;
                        let _: ::core::clone::AssertParamIsClone<usize>;
                        *self
                    }
                }
                #[automatically_derived]
                impl ::core::marker::Copy for PtrComponents {}
                let data = <*mut [u8]>::cast::<u8>(slice.as_ptr());
                let head_size =
                    <Self as ::vulkano::buffer::BufferContents>::LAYOUT.head_size() as usize;
                let element_size = <Self as ::vulkano::buffer::BufferContents>::LAYOUT
                    .element_size()
                    .unwrap_or(1) as usize;
                if true {
                    if !(slice.len() >= head_size) {
                        ::core::panicking::panic("assertion failed: slice.len() >= head_size")
                    }
                }
                let tail_size = slice.len() - head_size;
                if true {
                    if !(tail_size % element_size == 0) {
                        ::core::panicking::panic("assertion failed: tail_size % element_size == 0")
                    }
                }
                let len = tail_size / element_size;
                let components = PtrComponents { data, len };
                PtrRepr { components }.ptr
            }
        }
        #[automatically_derived]
        #[allow(non_camel_case_types, non_snake_case)]
        impl ::core::clone::Clone for UBO {
            #[inline]
            fn clone(&self) -> UBO {
                let _: ::core::clone::AssertParamIsClone<[f32; 4usize]>;
                *self
            }
        }
        #[automatically_derived]
        #[allow(non_camel_case_types, non_snake_case)]
        impl ::core::marker::Copy for UBO {}
    }
}
pub mod render {
    use log::info;
    use std::{collections::HashMap, env, sync::Arc};
    use vulkano::{
        command_buffer::allocator::{
            StandardCommandBufferAllocator, StandardCommandBufferAllocatorCreateInfo,
        },
        descriptor_set::allocator::StandardDescriptorSetAllocator,
        device::{
            physical::{PhysicalDevice, PhysicalDeviceType},
            Device, DeviceCreateInfo, DeviceExtensions, Features, Queue, QueueCreateInfo,
            QueueFlags,
        },
        format::Format,
        image::{view::ImageView, Image, ImageCreateInfo, ImageType, ImageUsage},
        instance::{Instance, InstanceCreateFlags, InstanceCreateInfo},
        memory::allocator::{AllocationCreateInfo, StandardMemoryAllocator},
        swapchain::{
            self, PresentMode, Surface, Swapchain, SwapchainCreateInfo, SwapchainPresentInfo,
        },
        sync::{self, GpuFuture},
        Validated, VulkanError, VulkanLibrary,
    };
    use winit::window::Window;
    pub struct Renderer {
        window: Arc<Window>,
        pub gfx_queue: Arc<Queue>,
        pub compute_queue: Arc<Queue>,
        swapchain: Arc<Swapchain>,
        final_views: Vec<Arc<ImageView>>,
        pub memory_allocator: Arc<StandardMemoryAllocator>,
        additional_image_views: HashMap<usize, Arc<ImageView>>,
        recreate_swapchain: bool,
        previous_frame_end: Option<Box<dyn GpuFuture>>,
        image_index: u32,
        present_mode: vulkano::swapchain::PresentMode,
        pub device: Arc<Device>,
        pub descriptor_set_allocator: Arc<StandardDescriptorSetAllocator>,
        pub command_buffer_allocator: Arc<StandardCommandBufferAllocator>,
        pub output_format: Format,
    }
    impl Renderer {
        pub fn new(window: Arc<winit::window::Window>) -> Renderer {
            let library = VulkanLibrary::new().expect("no local Vulkan library/DLL");
            let required_extensions = Surface::required_extensions(&window);
            let instance = Instance::new(
                library,
                InstanceCreateInfo {
                    flags: InstanceCreateFlags::ENUMERATE_PORTABILITY,
                    enabled_extensions: required_extensions,
                    ..Default::default()
                },
            )
            .expect("failed to create instance");
            let surface = Surface::from_window(instance.clone(), window.clone()).unwrap();
            let device_extensions = DeviceExtensions {
                khr_swapchain: true,
                ..DeviceExtensions::empty()
            };
            let (physical_device, _) = instance
                .enumerate_physical_devices()
                .unwrap()
                .filter(|p| p.supported_extensions().contains(&device_extensions))
                .filter_map(|p| {
                    p.queue_family_properties()
                        .iter()
                        .enumerate()
                        .position(|(i, q)| {
                            q.queue_flags.intersects(QueueFlags::GRAPHICS)
                                && p.surface_support(i as u32, &surface).unwrap_or(false)
                        })
                        .map(|i| (p, i as u32))
                })
                .min_by_key(|(p, _)| match p.properties().device_type {
                    PhysicalDeviceType::DiscreteGpu => 0,
                    PhysicalDeviceType::IntegratedGpu => 1,
                    PhysicalDeviceType::VirtualGpu => 2,
                    PhysicalDeviceType::Cpu => 3,
                    PhysicalDeviceType::Other => 4,
                    _ => 5,
                })
                .expect("no suitable physical device found");
            {
                let lvl = ::log::Level::Info;
                if lvl <= ::log::STATIC_MAX_LEVEL && lvl <= ::log::max_level() {
                    ::log::__private_api::log(
                        format_args!(
                            "Using device: {0} (type: {1:?})",
                            physical_device.properties().device_name,
                            physical_device.properties().device_type,
                        ),
                        lvl,
                        &(
                            "vulkan_engine::render",
                            "vulkan_engine::render",
                            ::log::__private_api::loc(),
                        ),
                        (),
                    );
                }
            };
            let (device, gfx_queue, compute_queue) =
                Self::create_device(physical_device, device_extensions, Default::default());
            let present_mode = if env::var("DisableVsync").is_ok() {
                vulkano::swapchain::PresentMode::Mailbox
            } else {
                vulkano::swapchain::PresentMode::Fifo
            };
            let (swapchain, final_views, output_format) =
                Self::create_swapchain(device.clone(), &window);
            let memory_allocator = Arc::new(StandardMemoryAllocator::new_default(device.clone()));
            let descriptor_set_allocator = Arc::new(StandardDescriptorSetAllocator::new(
                device.clone(),
                Default::default(),
            ));
            let previous_frame_end = Some(sync::now(device.clone()).boxed());
            let command_buffer_allocator = Arc::new(StandardCommandBufferAllocator::new(
                device.clone(),
                StandardCommandBufferAllocatorCreateInfo {
                    secondary_buffer_count: 32,
                    ..Default::default()
                },
            ));
            Renderer {
                window,
                gfx_queue,
                compute_queue,
                swapchain,
                final_views,
                memory_allocator,
                additional_image_views: HashMap::default(),
                recreate_swapchain: false,
                previous_frame_end,
                image_index: 0,
                present_mode,
                device,
                descriptor_set_allocator,
                command_buffer_allocator,
                output_format,
            }
        }
        fn create_device(
            physical_device: Arc<PhysicalDevice>,
            device_extensions: DeviceExtensions,
            features: Features,
        ) -> (Arc<Device>, Arc<Queue>, Arc<Queue>) {
            let queue_family_graphics = physical_device
                .queue_family_properties()
                .iter()
                .enumerate()
                .map(|(i, q)| (i as u32, q))
                .find(|(_i, q)| q.queue_flags.intersects(QueueFlags::GRAPHICS))
                .map(|(i, _)| i)
                .expect("could not find a queue that supports graphics");
            let queue_family_compute = physical_device
                .queue_family_properties()
                .iter()
                .enumerate()
                .map(|(i, q)| (i as u32, q))
                .find(|(i, q)| {
                    q.queue_flags.intersects(QueueFlags::COMPUTE) && *i != queue_family_graphics
                })
                .map(|(i, _)| i);
            let is_separate_compute_queue = false;
            let queue_create_infos = if let Some(queue_family_compute) = queue_family_compute {
                <[_]>::into_vec(
                    #[rustc_box]
                    ::alloc::boxed::Box::new([
                        QueueCreateInfo {
                            queue_family_index: queue_family_graphics,
                            ..Default::default()
                        },
                        QueueCreateInfo {
                            queue_family_index: queue_family_compute,
                            ..Default::default()
                        },
                    ]),
                )
            } else {
                <[_]>::into_vec(
                    #[rustc_box]
                    ::alloc::boxed::Box::new([QueueCreateInfo {
                        queue_family_index: queue_family_graphics,
                        ..Default::default()
                    }]),
                )
            };
            let (device, mut queues) = {
                Device::new(
                    physical_device,
                    DeviceCreateInfo {
                        queue_create_infos,
                        enabled_extensions: device_extensions,
                        enabled_features: features,
                        ..Default::default()
                    },
                )
                .expect("failed to create device")
            };
            let gfx_queue = queues.next().unwrap();
            let compute_queue = if is_separate_compute_queue {
                queues.next().unwrap()
            } else {
                gfx_queue.clone()
            };
            (device, gfx_queue, compute_queue)
        }
        /// Creates the swapchain and its images based on [`WindowDescriptor`]. The swapchain creation
        /// can be modified with the `swapchain_create_info_modify` function passed as an input.
        fn create_swapchain(
            device: Arc<Device>,
            window: &Arc<Window>,
        ) -> (Arc<Swapchain>, Vec<Arc<ImageView>>, Format) {
            let surface = Surface::from_window(device.instance().clone(), window.clone()).unwrap();
            let surface_capabilities = device
                .physical_device()
                .surface_capabilities(&surface, Default::default())
                .unwrap();
            let image_format = device
                .physical_device()
                .surface_formats(&surface, Default::default())
                .unwrap()[0]
                .0;
            let (swapchain, images) = Swapchain::new(device, surface, {
                let mut create_info = SwapchainCreateInfo {
                    min_image_count: surface_capabilities.min_image_count.max(2),
                    image_format,
                    image_extent: window.inner_size().into(),
                    image_usage: ImageUsage::COLOR_ATTACHMENT,
                    composite_alpha: surface_capabilities
                        .supported_composite_alpha
                        .into_iter()
                        .next()
                        .unwrap(),
                    ..Default::default()
                };
                create_info.present_mode = Self::create_swapchain_present_mode();
                create_info
            })
            .unwrap();
            let images = images
                .into_iter()
                .map(|image| ImageView::new_default(image).unwrap())
                .collect::<Vec<_>>();
            (swapchain, images, image_format)
        }
        fn create_swapchain_present_mode() -> PresentMode {
            if env::var("DisableVsync").is_ok() {
                vulkano::swapchain::PresentMode::Mailbox
            } else {
                vulkano::swapchain::PresentMode::Fifo
            }
        }
        pub fn swapchain_format(&self) -> Format {
            self.final_views[self.image_index as usize].format()
        }
        pub fn image_index(&self) -> u32 {
            self.image_index
        }
        pub fn gfx_queue(&self) -> Arc<Queue> {
            self.gfx_queue.clone()
        }
        pub fn compute_queue(&self) -> Arc<Queue> {
            self.compute_queue.clone()
        }
        pub fn surface(&self) -> Arc<Surface> {
            self.swapchain.surface().clone()
        }
        pub fn window(&self) -> &Window {
            &self.window
        }
        pub fn window_size(&self) -> [u32; 2] {
            let size = self.window().inner_size();
            [size.width, size.height]
        }
        pub fn swapchain_image_size(&self) -> [u32; 2] {
            self.final_views[0].image().extent()[0..2]
                .try_into()
                .unwrap()
        }
        pub fn swapchain_image_view(&self) -> Arc<ImageView> {
            self.final_views[self.image_index as usize].clone()
        }
        pub fn resolution(&self) -> [f32; 2] {
            let size = self.window().inner_size();
            let scale_factor = self.window().scale_factor();
            [
                (size.width as f64 / scale_factor) as f32,
                (size.height as f64 / scale_factor) as f32,
            ]
        }
        pub fn aspect_ratio(&self) -> f32 {
            let dims = self.window_size();
            dims[0] as f32 / dims[1] as f32
        }
        pub fn resize(&mut self) {
            self.recreate_swapchain = true;
        }
        pub fn add_additional_image_view(&mut self, key: usize, format: Format, usage: ImageUsage) {
            let final_view_image = self.final_views[0].image();
            let image = ImageView::new_default(
                Image::new(
                    self.memory_allocator.clone(),
                    ImageCreateInfo {
                        image_type: ImageType::Dim2d,
                        format,
                        extent: final_view_image.extent(),
                        usage,
                        ..Default::default()
                    },
                    AllocationCreateInfo::default(),
                )
                .unwrap(),
            )
            .unwrap();
            self.additional_image_views.insert(key, image);
        }
        pub fn get_additional_image_view(&mut self, key: usize) -> Arc<ImageView> {
            self.additional_image_views.get(&key).unwrap().clone()
        }
        pub fn remove_additional_image_view(&mut self, key: usize) {
            self.additional_image_views.remove(&key);
        }
        pub fn acquire(&mut self) -> Result<Box<dyn GpuFuture>, VulkanError> {
            if self.recreate_swapchain {
                self.recreate_swapchain_and_views();
            }
            let (image_index, suboptimal, acquire_future) =
                match swapchain::acquire_next_image(self.swapchain.clone(), None)
                    .map_err(Validated::unwrap)
                {
                    Ok(r) => r,
                    Err(VulkanError::OutOfDate) => {
                        self.recreate_swapchain = true;
                        return Err(VulkanError::OutOfDate);
                    }
                    Err(e) => {
                        ::core::panicking::panic_fmt(format_args!(
                            "failed to acquire next image: {0}",
                            e
                        ));
                    }
                };
            if suboptimal {
                self.recreate_swapchain = true;
            }
            self.image_index = image_index;
            let future = self.previous_frame_end.take().unwrap().join(acquire_future);
            Ok(future.boxed())
        }
        pub fn present(&mut self, after_future: Box<dyn GpuFuture>, wait_future: bool) {
            let future = after_future
                .then_swapchain_present(
                    self.gfx_queue.clone(),
                    SwapchainPresentInfo::swapchain_image_index(
                        self.swapchain.clone(),
                        self.image_index,
                    ),
                )
                .then_signal_fence_and_flush();
            match future.map_err(Validated::unwrap) {
                Ok(mut future) => {
                    if wait_future {
                        match future.wait(None) {
                            Ok(x) => x,
                            Err(e) => {
                                ::std::io::_print(format_args!("{0}\n", e));
                            }
                        }
                    } else {
                        future.cleanup_finished();
                    }
                    self.previous_frame_end = Some(future.boxed());
                }
                Err(VulkanError::OutOfDate) => {
                    self.recreate_swapchain = true;
                    self.previous_frame_end =
                        Some(sync::now(self.gfx_queue.device().clone()).boxed());
                }
                Err(e) => {
                    {
                        ::std::io::_print(format_args!("failed to flush future: {0}\n", e));
                    };
                    self.previous_frame_end =
                        Some(sync::now(self.gfx_queue.device().clone()).boxed());
                }
            }
        }
        fn recreate_swapchain_and_views(&mut self) {
            let image_extent: [u32; 2] = self.window().inner_size().into();
            if image_extent.contains(&0) {
                return;
            }
            let (new_swapchain, new_images) = self
                .swapchain
                .recreate(SwapchainCreateInfo {
                    image_extent,
                    present_mode: self.present_mode,
                    ..self.swapchain.create_info()
                })
                .expect("failed to recreate swapchain");
            self.swapchain = new_swapchain;
            let new_images = new_images
                .into_iter()
                .map(|image| ImageView::new_default(image).unwrap())
                .collect::<Vec<_>>();
            self.final_views = new_images;
            let resizable_views = self
                .additional_image_views
                .iter()
                .map(|c| *c.0)
                .collect::<Vec<usize>>();
            for i in resizable_views {
                let format = self.get_additional_image_view(i).format();
                let usage = self.get_additional_image_view(i).usage();
                self.remove_additional_image_view(i);
                self.add_additional_image_view(i, format, usage);
            }
            self.recreate_swapchain = false;
        }
    }
}
pub mod render_pass {
    use crate::{pixels_draw::PixelsDrawPipeline, render::Renderer};
    use ecolor::hex_color;
    use std::sync::Arc;
    use vulkano::{
        command_buffer::{
            allocator::StandardCommandBufferAllocator, AutoCommandBufferBuilder,
            CommandBufferUsage, RenderPassBeginInfo, SubpassBeginInfo, SubpassContents,
        },
        device::Queue,
        image::view::ImageView,
        render_pass::{Framebuffer, FramebufferCreateInfo, RenderPass, Subpass},
        sync::GpuFuture,
    };
    /// A render pass which places an incoming image over the frame, filling it.
    pub struct RenderPassPlaceOverFrame {
        gfx_queue: Arc<Queue>,
        render_pass: Arc<RenderPass>,
        pixels_draw_pipeline: PixelsDrawPipeline,
        command_buffer_allocator: Arc<StandardCommandBufferAllocator>,
    }
    impl RenderPassPlaceOverFrame {
        pub fn new(app: &Renderer) -> RenderPassPlaceOverFrame {
            let render_pass = {
                use ::vulkano::render_pass::RenderPass;
                let create_info = {
                    #[allow(unused)]
                    let mut attachment_num = 0;
                    let color = attachment_num;
                    attachment_num += 1;
                    #[allow(unused)]
                    struct Layouts {
                        initial_layout: Option<::vulkano::image::ImageLayout>,
                        final_layout: Option<::vulkano::image::ImageLayout>,
                    }
                    #[automatically_derived]
                    #[allow(unused)]
                    impl ::core::clone::Clone for Layouts {
                        #[inline]
                        fn clone(&self) -> Layouts {
                            let _: ::core::clone::AssertParamIsClone<
                                Option<::vulkano::image::ImageLayout>,
                            >;
                            let _: ::core::clone::AssertParamIsClone<
                                Option<::vulkano::image::ImageLayout>,
                            >;
                            *self
                        }
                    }
                    #[automatically_derived]
                    #[allow(unused)]
                    impl ::core::marker::Copy for Layouts {}
                    #[automatically_derived]
                    #[allow(unused)]
                    impl ::core::default::Default for Layouts {
                        #[inline]
                        fn default() -> Layouts {
                            Layouts {
                                initial_layout: ::core::default::Default::default(),
                                final_layout: ::core::default::Default::default(),
                            }
                        }
                    }
                    #[allow(unused)]
                    let mut layouts: Vec<Layouts> = ::alloc::vec::from_elem(
                        Layouts::default(),
                        attachment_num as usize,
                    );
                    let subpasses = <[_]>::into_vec(
                        #[rustc_box]
                        ::alloc::boxed::Box::new([
                            {
                                let desc = ::vulkano::render_pass::SubpassDescription {
                                    color_attachments: <[_]>::into_vec(
                                        #[rustc_box]
                                        ::alloc::boxed::Box::new([
                                            {
                                                let layouts = &mut layouts[color as usize];
                                                layouts.initial_layout = layouts
                                                    .initial_layout
                                                    .or(
                                                        Some(::vulkano::image::ImageLayout::ColorAttachmentOptimal),
                                                    );
                                                layouts.final_layout = Some(
                                                    ::vulkano::image::ImageLayout::ColorAttachmentOptimal,
                                                );
                                                Some(::vulkano::render_pass::AttachmentReference {
                                                    attachment: color,
                                                    layout: ::vulkano::image::ImageLayout::ColorAttachmentOptimal,
                                                    ..Default::default()
                                                })
                                            },
                                        ]),
                                    ),
                                    color_resolve_attachments: ::alloc::vec::Vec::new(),
                                    depth_stencil_attachment: { None },
                                    depth_stencil_resolve_attachment: { None },
                                    depth_resolve_mode: { None },
                                    stencil_resolve_mode: { None },
                                    input_attachments: ::alloc::vec::Vec::new(),
                                    preserve_attachments: (0..attachment_num)
                                        .filter(|&a| { ![color].contains(&a) })
                                        .collect(),
                                    ..Default::default()
                                };
                                desc
                            },
                        ]),
                    );
                    let dependencies: Vec<_> = (0..subpasses.len().saturating_sub(1)
                        as u32)
                        .map(|id| {
                            let src_stages = ::vulkano::sync::PipelineStages::ALL_GRAPHICS;
                            let dst_stages = ::vulkano::sync::PipelineStages::ALL_GRAPHICS;
                            let src_access = ::vulkano::sync::AccessFlags::MEMORY_READ
                                | ::vulkano::sync::AccessFlags::MEMORY_WRITE;
                            let dst_access = ::vulkano::sync::AccessFlags::MEMORY_READ
                                | ::vulkano::sync::AccessFlags::MEMORY_WRITE;
                            ::vulkano::render_pass::SubpassDependency {
                                src_subpass: id.into(),
                                dst_subpass: (id + 1).into(),
                                src_stages,
                                dst_stages,
                                src_access,
                                dst_access,
                                dependency_flags: ::vulkano::sync::DependencyFlags::BY_REGION,
                                ..Default::default()
                            }
                        })
                        .collect();
                    let attachments = <[_]>::into_vec(
                        #[rustc_box]
                        ::alloc::boxed::Box::new([
                            {
                                let layouts = &mut layouts[color as usize];
                                ::vulkano::render_pass::AttachmentDescription {
                                    format: app.output_format,
                                    samples: ::vulkano::image::SampleCount::try_from(1)
                                        .unwrap(),
                                    load_op: ::vulkano::render_pass::AttachmentLoadOp::Clear,
                                    store_op: ::vulkano::render_pass::AttachmentStoreOp::Store,
                                    initial_layout: layouts
                                        .initial_layout
                                        .expect(
                                            ::alloc::__export::must_use({
                                                    let res = ::alloc::fmt::format(
                                                        format_args!(
                                                            "Attachment {0} is missing initial_layout, this is normally automatically determined but you can manually specify it for an individual attachment in the single_pass_renderpass! macro",
                                                            attachment_num,
                                                        ),
                                                    );
                                                    res
                                                })
                                                .as_ref(),
                                        ),
                                    final_layout: layouts
                                        .final_layout
                                        .expect(
                                            ::alloc::__export::must_use({
                                                    let res = ::alloc::fmt::format(
                                                        format_args!(
                                                            "Attachment {0} is missing final_layout, this is normally automatically determined but you can manually specify it for an individual attachment in the single_pass_renderpass! macro",
                                                            attachment_num,
                                                        ),
                                                    );
                                                    res
                                                })
                                                .as_ref(),
                                        ),
                                    ..Default::default()
                                }
                            },
                        ]),
                    );
                    ::vulkano::render_pass::RenderPassCreateInfo {
                        attachments,
                        subpasses,
                        dependencies,
                        ..Default::default()
                    }
                };
                RenderPass::new(app.gfx_queue.device().clone(), create_info)
            }
                .unwrap();
            let subpass = Subpass::from(render_pass.clone(), 0).unwrap();
            let pixels_draw_pipeline = PixelsDrawPipeline::new(app, subpass);
            let gfx_queue = app.gfx_queue();
            RenderPassPlaceOverFrame {
                gfx_queue,
                render_pass,
                pixels_draw_pipeline,
                command_buffer_allocator: app.command_buffer_allocator.clone(),
            }
        }
        /// Places the view exactly over the target swapchain image. The texture draw pipeline uses a
        /// quad onto which it places the view.
        pub fn render<F>(
            &self,
            before_future: F,
            image_view: Arc<ImageView>,
            target: Arc<ImageView>,
            background_color: [f32; 4],
        ) -> Box<dyn GpuFuture>
        where
            F: GpuFuture + 'static,
        {
            let img_dims: [u32; 2] = target.image().extent()[0..2].try_into().unwrap();
            let framebuffer = Framebuffer::new(
                self.render_pass.clone(),
                FramebufferCreateInfo {
                    attachments: <[_]>::into_vec(
                        #[rustc_box]
                        ::alloc::boxed::Box::new([target]),
                    ),
                    ..Default::default()
                },
            )
            .unwrap();
            let mut command_buffer_builder = AutoCommandBufferBuilder::primary(
                self.command_buffer_allocator.as_ref(),
                self.gfx_queue.queue_family_index(),
                CommandBufferUsage::OneTimeSubmit,
            )
            .unwrap();
            command_buffer_builder
                .begin_render_pass(
                    RenderPassBeginInfo {
                        clear_values: <[_]>::into_vec(
                            #[rustc_box]
                            ::alloc::boxed::Box::new([Some(background_color.into())]),
                        ),
                        ..RenderPassBeginInfo::framebuffer(framebuffer)
                    },
                    SubpassBeginInfo {
                        contents: SubpassContents::SecondaryCommandBuffers,
                        ..Default::default()
                    },
                )
                .unwrap();
            let cb = self
                .pixels_draw_pipeline
                .draw(img_dims, image_view, background_color);
            command_buffer_builder.execute_commands(cb).unwrap();
            command_buffer_builder
                .end_render_pass(Default::default())
                .unwrap();
            let command_buffer = command_buffer_builder.build().unwrap();
            let after_future = before_future
                .then_execute(self.gfx_queue.clone(), command_buffer)
                .unwrap();
            after_future.boxed()
        }
    }
}
pub mod state {
    use ecolor::hex_color;
    use input::InputState;
    use std::sync::Arc;
    use winit::{event::WindowEvent, event_loop::ActiveEventLoop, window::Window};
    pub struct RenderPipeline {
        pub compute: SandComputePipeline,
        pub place_over_frame: RenderPassPlaceOverFrame,
    }
    impl RenderPipeline {
        pub fn new(renderer: &Renderer) -> RenderPipeline {
            RenderPipeline {
                compute: SandComputePipeline::new(renderer),
                place_over_frame: RenderPassPlaceOverFrame::new(renderer),
            }
        }
    }
    pub struct State {
        pub render_pipeline: RenderPipeline,
        renderer: Renderer,
        gui: GameGui,
        pub sim_clock: SimClock,
        input: InputState,
        selected_cell_type: CellType,
        background_color: [f32; 4],
    }
    impl State {
        pub async fn new(window: Arc<Window>, event_loop: &ActiveEventLoop) -> State {
            let renderer = Renderer::new(window);
            let render_pipeline = RenderPipeline::new(&renderer);
            let gui = GameGui::new(
                event_loop,
                renderer.surface(),
                renderer.gfx_queue.clone(),
                renderer.output_format,
            );
            let sim_clock = SimClock::default();
            State {
                renderer,
                render_pipeline,
                gui,
                sim_clock,
                input: InputState::default(),
                selected_cell_type: CellType::Sand,
                background_color: {
                    let array = [143u8, 163u8, 179u8];
                    if array.len() == 3 {
                        ::ecolor::Color32::from_rgb(array[0], array[1], array[2])
                    } else {
                        #[allow(unconditional_panic)]
                        ::ecolor::Color32::from_rgba_unmultiplied(
                            array[0], array[1], array[2], array[3],
                        )
                    }
                }
                .to_normalized_gamma_f32(),
            }
        }
        pub fn render(&mut self) {
            self.sim_clock.clock();
            self.gui.draw_gui(
                &mut self.sim_clock,
                &mut self.render_pipeline.compute,
                &mut self.input.mouse.hover_gui,
                &mut self.selected_cell_type,
                self.renderer.window_size(),
                &mut self.background_color,
            );
            if self.input.mouse.left_pressed && !self.input.mouse.hover_gui {
                self.render_pipeline.compute.draw(
                    self.input.mouse.position,
                    self.renderer.window_size(),
                    self.selected_cell_type,
                );
            }
            if self.input.mouse.right_pressed && !self.input.mouse.hover_gui {
                self.render_pipeline.compute.draw(
                    self.input.mouse.position,
                    self.renderer.window_size(),
                    CellType::Empty,
                );
            }
            let before_pipeline_future = match self.renderer.acquire() {
                Err(e) => {
                    {
                        ::std::io::_print(format_args!("{0}\n", e));
                    };
                    return;
                }
                Ok(future) => future,
            };
            let after_compute = self
                .render_pipeline
                .compute
                .compute(before_pipeline_future, self.sim_clock.simulate());
            let color_image = self.render_pipeline.compute.color_image();
            let target_image = self.renderer.swapchain_image_view();
            let after_render = self.render_pipeline.place_over_frame.render(
                after_compute,
                color_image,
                target_image.clone(),
                self.background_color,
            );
            let after_gui = self.gui.draw_on_image(after_render, target_image);
            self.renderer.present(after_gui, true);
        }
        pub fn resize(&mut self, size: [u32; 2]) {
            self.renderer.resize();
            self.render_pipeline.compute.resize(size)
        }
        pub fn event(&mut self, ev: WindowEvent) {
            self.gui.event(&ev);
            self.input.event(ev);
        }
    }
    use crate::{
        compute_sand::{CellType, SandComputePipeline},
        gui::GameGui,
        render::Renderer,
        render_pass::RenderPassPlaceOverFrame,
    };
    pub struct SimClock {
        simulate: bool,
        simulate_ui_togle: bool,
        sim_rate: u16,
        cur_sim: u16,
    }
    impl Default for SimClock {
        fn default() -> Self {
            SimClock {
                simulate: true,
                simulate_ui_togle: true,
                sim_rate: 1,
                cur_sim: 1,
            }
        }
    }
    impl SimClock {
        pub fn clock(&mut self) {
            if self.cur_sim == self.sim_rate {
                self.simulate = true;
                self.sim_rate = 0;
            } else if self.simulate_ui_togle {
                self.simulate = false;
                self.sim_rate += 1;
            }
            if !self.simulate_ui_togle {
                self.simulate = false;
            }
        }
        pub fn ui_togles(&mut self) -> (&mut bool, &mut u16, &mut u16) {
            (
                &mut self.simulate_ui_togle,
                &mut self.cur_sim,
                &mut self.sim_rate,
            )
        }
        fn simulate(&mut self) -> bool {
            self.simulate
        }
    }
    pub mod camera {
        pub struct Camera {}
    }
    pub mod input {
        use glam::Vec2;
        use log::info;
        use winit::event::{ElementState, MouseButton, WindowEvent};
        pub struct InputState {
            pub mouse: MouseState,
            pub keyboard: KeyboardState,
        }
        #[automatically_derived]
        impl ::core::default::Default for InputState {
            #[inline]
            fn default() -> InputState {
                InputState {
                    mouse: ::core::default::Default::default(),
                    keyboard: ::core::default::Default::default(),
                }
            }
        }
        impl InputState {
            pub fn event(&mut self, ev: WindowEvent) {
                self.mouse.event(ev);
            }
        }
        pub struct KeyboardState {}
        #[automatically_derived]
        impl ::core::default::Default for KeyboardState {
            #[inline]
            fn default() -> KeyboardState {
                KeyboardState {}
            }
        }
        pub struct MouseState {
            pub position: Vec2,
            pub left_pressed: bool,
            pub right_pressed: bool,
            pub hover_gui: bool,
        }
        #[automatically_derived]
        impl ::core::default::Default for MouseState {
            #[inline]
            fn default() -> MouseState {
                MouseState {
                    position: ::core::default::Default::default(),
                    left_pressed: ::core::default::Default::default(),
                    right_pressed: ::core::default::Default::default(),
                    hover_gui: ::core::default::Default::default(),
                }
            }
        }
        impl MouseState {
            pub fn event(&mut self, ev: WindowEvent) {
                match ev {
                    WindowEvent::MouseInput { state, button, .. } => match (state, button) {
                        (ElementState::Pressed, MouseButton::Left) => {
                            self.left_pressed = true;
                            {
                                let lvl = ::log::Level::Info;
                                if lvl <= ::log::STATIC_MAX_LEVEL && lvl <= ::log::max_level() {
                                    ::log::__private_api::log(
                                        format_args!("mouse pressed"),
                                        lvl,
                                        &(
                                            "vulkan_engine::state::input",
                                            "vulkan_engine::state::input",
                                            ::log::__private_api::loc(),
                                        ),
                                        (),
                                    );
                                }
                            };
                        }
                        M(ElementState::Released, MouseButton::Left) => {
                            self.left_pressed = false;
                            {
                                let lvl = ::log::Level::Info;
                                if lvl <= ::log::STATIC_MAX_LEVEL && lvl <= ::log::max_level() {
                                    ::log::__private_api::log(
                                        format_args!("mouse released"),
                                        lvl,
                                        &(
                                            "vulkan_engine::state::input",
                                            "vulkan_engine::state::input",
                                            ::log::__private_api::loc(),
                                        ),
                                        (),
                                    );
                                }
                            };
                        }
                        (ElementState::Pressed, MouseButton::Right) => {
                            self.right_pressed = true;
                            {
                                let lvl = ::log::Level::Info;
                                if lvl <= ::log::STATIC_MAX_LEVEL && lvl <= ::log::max_level() {
                                    ::log::__private_api::log(
                                        format_args!("mouse pressed"),
                                        lvl,
                                        &(
                                            "vulkan_engine::state::input",
                                            "vulkan_engine::state::input",
                                            ::log::__private_api::loc(),
                                        ),
                                        (),
                                    );
                                }
                            };
                        }
                        (ElementState::Released, MouseButton::Right) => {
                            self.right_pressed = false;
                            {
                                let lvl = ::log::Level::Info;
                                if lvl <= ::log::STATIC_MAX_LEVEL && lvl <= ::log::max_level() {
                                    ::log::__private_api::log(
                                        format_args!("mouse released"),
                                        lvl,
                                        &(
                                            "vulkan_engine::state::input",
                                            "vulkan_engine::state::input",
                                            ::log::__private_api::loc(),
                                        ),
                                        (),
                                    );
                                }
                            };
                        }
                        _ => {}
                    },
                    WindowEvent::CursorMoved { position, .. } => {
                        self.position = Vec2::new(position.x as f32, position.y as f32);
                    }
                    _ => {}
                }
            }
        }
    }
}
use app::App;
use log::info;
use winit::event_loop::EventLoop;
fn main() {
    env_logger::init();
    {
        let lvl = ::log::Level::Info;
        if lvl <= ::log::STATIC_MAX_LEVEL && lvl <= ::log::max_level() {
            ::log::__private_api::log(
                format_args!("Creating event loop"),
                lvl,
                &(
                    "vulkan_engine",
                    "vulkan_engine",
                    ::log::__private_api::loc(),
                ),
                (),
            );
        }
    };
    let event_loop = EventLoop::new().unwrap();
    let mut app = App::default();
    event_loop.run_app(&mut app).unwrap();
}
