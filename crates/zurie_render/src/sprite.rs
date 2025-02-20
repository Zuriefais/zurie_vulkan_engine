use anyhow::Ok;
use asefile::AsepriteFile;
use egui_winit_vulkano::egui::load::SizedTexture;
use egui_winit_vulkano::egui::{self, ColorImage, Context, TextureHandle};
use log::info;
use slotmap::SlotMap;
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
    image::{Image, ImageCreateInfo, ImageType, ImageUsage, view::ImageView},
    memory::allocator::{AllocationCreateInfo, StandardMemoryAllocator},
};
use zurie_shared::slotmap::Key;
use zurie_types::SpriteHandle;

#[derive(Debug)]
pub enum LoadSpriteInfo {
    Path(Box<Path>),
    Buffer(Vec<u8>),
}

pub struct SpriteManager {
    sprites: SlotMap<SpriteHandle, Option<Sprite>>,
    to_load_queue: Vec<(SpriteHandle, LoadSpriteInfo)>,
    error_sprite: SpriteHandle,
    egui_context: Context,
}

impl SpriteManager {
    pub fn new(egui_context: Context) -> Self {
        let mut sprites: SlotMap<SpriteHandle, Option<Sprite>> = Default::default();
        let error_sprite = sprites.insert(None);
        Self {
            sprites,
            to_load_queue: vec![(
                error_sprite,
                LoadSpriteInfo::Buffer(include_bytes!("../../../static/error.aseprite").to_vec()),
            )],
            error_sprite,
            egui_context,
        }
    }
    pub fn gui(&mut self) {
        egui::Window::new("Sprite manager").show(&self.egui_context, |ctx| {
            for sprite in self.sprites.iter() {
                if let Some(sprite) = &sprite.1.as_ref() {
                    ctx.image(SizedTexture::from_handle(&sprite.egui_texture_handle));
                }
            }
        });
    }

    pub fn push_to_load_queue(&mut self, to_load: LoadSpriteInfo) -> SpriteHandle {
        let handle = self.sprites.insert(None);
        info!(
            "Sprite added to load queue, {:?} {}",
            to_load,
            handle.data().as_ffi()
        );
        self.to_load_queue.push((handle, to_load));

        handle
    }

    pub fn process_queue(
        &mut self,
        memory_allocator: Arc<StandardMemoryAllocator>,
        command_buffer_allocator: Arc<StandardCommandBufferAllocator>,
        queue: Arc<Queue>,
    ) -> anyhow::Result<()> {
        if !self.to_load_queue.is_empty() {
            info!("starting processing sprites from queue");
        }
        for (handle, to_load) in self.to_load_queue.drain(..) {
            if let Some(slot) = self.sprites.get_mut(handle) {
                *slot = match to_load {
                    LoadSpriteInfo::Path(path) => {
                        info!("processing {:?}", handle);
                        Some(Sprite::from_file(
                            &path,
                            memory_allocator.clone(),
                            command_buffer_allocator.clone(),
                            queue.clone(),
                            self.egui_context.clone(),
                        )?)
                    }
                    LoadSpriteInfo::Buffer(buf) => Some(Sprite::from_buffer(
                        &buf,
                        memory_allocator.clone(),
                        command_buffer_allocator.clone(),
                        queue.clone(),
                        self.egui_context.clone(),
                    )?),
                }
            }
        }
        Ok(())
    }

    pub fn get_texture(&self, handle: SpriteHandle) -> Option<Arc<ImageView>> {
        let sprite = self.sprites.get(handle);
        let result = sprite
            .and_then(|sprite| sprite.as_ref())
            .map(|sprite| sprite.texture.clone());

        if result.is_none() {
            log::warn!(
                "Failed to get texture for sprite handle {:?}",
                handle.data().as_ffi()
            );
            return self
                .sprites
                .get(self.error_sprite)
                .and_then(|sprite| sprite.as_ref())
                .map(|sprite| sprite.texture.clone());
        }

        result
    }

    pub fn get_sprite(&self, handle: SpriteHandle) -> &Option<Sprite> {
        if let Some(sprite) = self.sprites.get(handle) {
            return sprite;
        }
        &None
    }

    pub fn load_from_file(
        &mut self,
        path: &Path,
        memory_allocator: Arc<StandardMemoryAllocator>,
        command_buffer_allocator: Arc<StandardCommandBufferAllocator>,
        queue: Arc<Queue>,
    ) -> anyhow::Result<SpriteHandle> {
        Ok(self.sprites.insert(Some(Sprite::from_file(
            path,
            memory_allocator,
            command_buffer_allocator,
            queue,
            self.egui_context.clone(),
        )?)))
    }

    pub fn load_from_buffer(
        &mut self,
        buffer: &[u8],
        memory_allocator: Arc<StandardMemoryAllocator>,
        command_buffer_allocator: Arc<StandardCommandBufferAllocator>,
        queue: Arc<Queue>,
    ) -> anyhow::Result<SpriteHandle> {
        Ok(self.sprites.insert(Some(Sprite::from_buffer(
            buffer,
            memory_allocator,
            command_buffer_allocator,
            queue,
            self.egui_context.clone(),
        )?)))
    }

    pub fn reload_sprites(
        &mut self,
        memory_allocator: Arc<StandardMemoryAllocator>,
        command_buffer_allocator: Arc<StandardCommandBufferAllocator>,
        queue: Arc<Queue>,
    ) -> anyhow::Result<()> {
        for sprite in self.sprites.iter_mut() {
            if let Some(sprite_data) = sprite.1.as_ref() {
                if let Some(path) = &sprite_data.path {
                    *sprite.1 = Some(Sprite::from_file(
                        Path::new(path),
                        memory_allocator.clone(),
                        command_buffer_allocator.clone(),
                        queue.clone(),
                        self.egui_context.clone(),
                    )?);
                }
            }
        }
        Ok(())
    }
}

pub struct Sprite {
    pub texture: Arc<ImageView>,
    pub width: u32,
    pub height: u32,
    pub path: Option<String>,
    pub egui_texture_handle: TextureHandle,
}

impl Sprite {
    pub fn from_buffer(
        buffer: &[u8],
        memory_allocator: Arc<StandardMemoryAllocator>,
        command_buffer_allocator: Arc<StandardCommandBufferAllocator>,
        queue: Arc<Queue>,
        ctx: Context,
    ) -> anyhow::Result<Self> {
        let ase = AsepriteFile::read(buffer)?;
        let (texture, width, height, handle) =
            texture_from_ase(ase, memory_allocator, command_buffer_allocator, queue, ctx)?;
        Ok(Self {
            texture,
            width,
            height,
            path: None,
            egui_texture_handle: handle,
        })
    }
    pub fn from_file(
        path: &Path,
        memory_allocator: Arc<StandardMemoryAllocator>,
        command_buffer_allocator: Arc<StandardCommandBufferAllocator>,
        queue: Arc<Queue>,
        ctx: Context,
    ) -> anyhow::Result<Self> {
        info!("Loading sprite from {:?}", path);
        let ase = AsepriteFile::read_file(path)?;

        let path_str = path.to_str().expect("Error getting path").to_string();
        let (texture, width, height, handle) =
            texture_from_ase(ase, memory_allocator, command_buffer_allocator, queue, ctx)?;
        info!("sprite loaded");
        Ok(Self {
            texture,
            width,
            height,
            path: Some(path_str),
            egui_texture_handle: handle,
        })
    }
    pub fn texture(&self) -> Arc<ImageView> {
        self.texture.clone()
    }
}

fn texture_from_ase(
    ase: AsepriteFile,
    memory_allocator: Arc<StandardMemoryAllocator>,
    command_buffer_allocator: Arc<StandardCommandBufferAllocator>,
    queue: Arc<Queue>,
    ctx: Context,
) -> anyhow::Result<(Arc<ImageView>, u32, u32, TextureHandle)> {
    let frame = ase.frame(0).image();

    let width = frame.width();
    let height = frame.height();
    let rgba_data: Vec<u8> = frame
        .pixels()
        .flat_map(|p| [p[0], p[1], p[2], p[3]])
        .collect();
    let egui_handle = ctx.load_texture(
        "",
        ColorImage::from_rgba_unmultiplied(
            [frame.width() as usize, frame.height() as usize],
            &rgba_data,
        ),
        Default::default(),
    );
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

    Ok((texture, width, height, egui_handle))
}
