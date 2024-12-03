use asefile::AsepriteFile;
use std::{path::Path, sync::Arc};
use vulkano::buffer::{Buffer, BufferCreateInfo, BufferUsage};
use vulkano::command_buffer::allocator::StandardCommandBufferAllocator;
use vulkano::command_buffer::{
    AutoCommandBufferBuilder, CommandBufferUsage, CopyBufferToImageInfo,
    PrimaryCommandBufferAbstract,
};
use vulkano::device::Queue;
use vulkano::memory::allocator::MemoryTypeFilter;
use vulkano::sync::GpuFuture;
use vulkano::{
    format::Format,
    image::{view::ImageView, Image, ImageCreateInfo, ImageType, ImageUsage},
    memory::allocator::{AllocationCreateInfo, StandardMemoryAllocator},
};

pub struct Sprite {
    pub texture: Arc<ImageView>,
    pub width: u32,
    pub height: u32,
}

impl Sprite {
    pub fn from_aseprite(
        path: &Path,
        memory_allocator: Arc<StandardMemoryAllocator>,
        command_buffer_allocator: Arc<StandardCommandBufferAllocator>,
        queue: Arc<Queue>,
    ) -> anyhow::Result<Self> {
        let ase = AsepriteFile::read_file(path)?;
        let frame = ase.frame(0).image();

        let width = frame.width();
        let height = frame.height();
        let rgba_data: Vec<u8> = frame
            .pixels()
            .flat_map(|p| [p[0], p[1], p[2], p[3]])
            .collect();

        // Create a buffer with the pixel data
        let upload_buffer = Buffer::from_iter(
            memory_allocator.clone(),
            BufferCreateInfo {
                usage: BufferUsage::TRANSFER_SRC,
                ..Default::default()
            },
            AllocationCreateInfo {
                memory_type_filter: MemoryTypeFilter::PREFER_HOST
                    | MemoryTypeFilter::HOST_SEQUENTIAL_WRITE,
                ..Default::default()
            },
            rgba_data,
        )?;

        // Create the image
        let image = Image::new(
            memory_allocator,
            ImageCreateInfo {
                image_type: ImageType::Dim2d,
                format: Format::R8G8B8A8_UNORM,
                extent: [width, height, 1],
                usage: ImageUsage::TRANSFER_DST | ImageUsage::SAMPLED,
                ..Default::default()
            },
            AllocationCreateInfo {
                memory_type_filter: MemoryTypeFilter::PREFER_DEVICE,
                ..Default::default()
            },
        )?;

        // Create command buffer to copy buffer to image
        let mut builder = AutoCommandBufferBuilder::primary(
            command_buffer_allocator.as_ref(),
            queue.queue_family_index(),
            CommandBufferUsage::OneTimeSubmit,
        )?;

        builder.copy_buffer_to_image(CopyBufferToImageInfo::buffer_image(
            upload_buffer,
            image.clone(),
        ))?;

        let command_buffer = builder.build()?;
        let future = command_buffer.execute(queue)?;
        // Wait for the GPU to finish
        future.then_signal_fence_and_flush()?.wait(None)?;

        let texture = ImageView::new_default(image)?;

        Ok(Self {
            texture,
            width,
            height,
        })
    }
    pub fn texture(&self) -> Arc<ImageView> {
        self.texture.clone()
    }
}
