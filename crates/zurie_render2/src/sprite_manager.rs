use anyhow::Ok;
use asefile::AsepriteFile;
use ash::vk;
use crossbeam_channel::{Receiver, Sender, unbounded};
use log::info;
use slotmap::{Key, SlotMap};
use std::path::Path;
use std::sync::{Arc, RwLock};
use std::thread::{self, JoinHandle};
use zurie_render_glue::LoadSpriteInfo as ZurieLoadSpriteInfo;
use zurie_types::SpriteHandle;

use crate::render::backend::Backend;
use crate::utils::vulkan_init::{begin_single_time_commands, end_single_time_commands};

pub struct SpriteManagerBackend {
    sprites: Arc<RwLock<SlotMap<SpriteHandle, Option<Sprite>>>>,
    queue: Arc<RwLock<Vec<(SpriteHandle, LoadSpriteInfo)>>>,
    receiver: Receiver<SpriteManagerCommand>,
    sender: Sender<SpriteManagerCommand>,
    error_sprite: SpriteHandle,
    command_thread: Option<JoinHandle<()>>,
}

pub struct SpriteManager {
    sender: Sender<SpriteManagerCommand>,
}

pub enum SpriteManagerCommand {
    LoadSprite(LoadSpriteInfo, Sender<SpriteHandle>),
}

#[derive(Debug)]
pub enum LoadSpriteInfo {
    Path(Box<Path>),
    Buffer(Vec<u8>),
}

#[derive(Clone)]
pub struct Sprite {
    pub texture: Arc<vk::ImageView>,
    pub image: vk::Image,
    pub memory: vk::DeviceMemory,
    pub width: u32,
    pub height: u32,
    pub path: Option<String>,
}

impl Drop for Sprite {
    fn drop(&mut self) {
        // Cleanup Vulkan resources
        // Note: Actual cleanup should be handled by the backend when the sprite is removed
    }
}

impl SpriteManagerBackend {
    pub fn new(backend: &Backend) -> anyhow::Result<Self> {
        let (sender, receiver) = unbounded();

        let mut sprites = SlotMap::default();
        let error_handle = sprites.insert(None);
        let error_sprite = Sprite::create_error_sprite(backend)?;
        sprites[error_handle] = Some(error_sprite);

        let sprites = Arc::new(RwLock::new(sprites));
        let queue = Arc::new(RwLock::new(Vec::new()));

        // Clone for thread
        let thread_sprites = sprites.clone();
        let thread_queue = queue.clone();
        let thread_receiver = receiver.clone();

        // Spawn a dedicated thread to process commands
        let command_thread = thread::spawn(move || {
            while let std::result::Result::Ok(command) = thread_receiver.recv() {
                match command {
                    SpriteManagerCommand::LoadSprite(load_info, reply_sender) => {
                        match thread_sprites.write() {
                            std::result::Result::Ok(mut sprites) => {
                                let handle = sprites.insert(None); // Reserve slot
                                match thread_queue.write() {
                                    std::result::Result::Ok(mut queue) => {
                                        queue.push((handle, load_info));
                                        // Send handle back to caller
                                        if let Err(e) = reply_sender.send(handle) {
                                            log::error!("Failed to send sprite handle: {}", e);
                                        }
                                    }
                                    Err(e) => {
                                        log::error!("Queue RwLock poisoned: {}", e);
                                    }
                                }
                            }
                            Err(e) => {
                                log::error!("Sprites RwLock poisoned: {}", e);
                            }
                        }
                    }
                }
            }
            log::info!("Sprite command thread shutting down");
        });

        Ok(Self {
            sprites,
            queue,
            receiver,
            sender,
            error_sprite: error_handle,
            command_thread: Some(command_thread),
        })
    }

    pub fn get_manager(&self) -> SpriteManager {
        SpriteManager::new(self.sender.clone())
    }

    pub fn process_queue(&mut self, backend: &Backend) -> anyhow::Result<()> {
        // Process queued sprite loading tasks
        let mut queue = self
            .queue
            .write()
            .map_err(|e| anyhow::anyhow!("RwLock poisoned: {}", e))?;
        if !queue.is_empty() {
            info!("Processing {} sprites from queue", queue.len());
        }

        // Limit processing to avoid frame spikes
        const MAX_SPRITES_PER_FRAME: usize = 5;
        let to_process = queue
            .drain(..)
            .take(MAX_SPRITES_PER_FRAME)
            .collect::<Vec<_>>();
        drop(queue); // Release lock early

        for (handle, to_load) in to_process {
            let mut sprites = self
                .sprites
                .write()
                .map_err(|e| anyhow::anyhow!("RwLock poisoned: {}", e))?;
            if let Some(slot) = sprites.get_mut(handle) {
                *slot = match to_load {
                    LoadSpriteInfo::Path(path) => {
                        info!("Loading sprite from path: {:?}", path);
                        Some(Sprite::from_file(&path, backend)?)
                    }
                    LoadSpriteInfo::Buffer(buf) => {
                        info!("Loading sprite from buffer");
                        Some(Sprite::from_buffer(&buf, backend)?)
                    }
                };
            } else {
                log::warn!(
                    "Sprite handle {:?} not found during queue processing",
                    handle.data().as_ffi()
                );
            }
        }
        Ok(())
    }

    pub fn get_texture(&self, handle: SpriteHandle) -> Option<Arc<vk::ImageView>> {
        let sprites = self.sprites.read().ok()?;
        let sprite = sprites.get(handle);
        let result = sprite
            .and_then(|sprite| sprite.as_ref())
            .map(|sprite| sprite.texture.clone());

        if result.is_none() {
            log::warn!(
                "Failed to get texture for sprite handle {:?}",
                handle.data().as_ffi()
            );
            return sprites
                .get(self.error_sprite)
                .and_then(|sprite| sprite.as_ref())
                .map(|sprite| sprite.texture.clone());
        }
        result
    }

    pub fn get_sprite(&self, handle: SpriteHandle) -> Option<Arc<Option<Sprite>>> {
        let sprites = self.sprites.read().ok()?;
        sprites.get(handle).map(|sprite| Arc::new(sprite.clone()))
    }

    pub fn cleanup(&self, backend: &Backend) {
        let sprites = match self.sprites.write() {
            std::result::Result::Ok(sprites) => sprites,
            Err(e) => {
                log::error!("Failed to acquire sprite lock for cleanup: {}", e);
                return;
            }
        };
        for (_, sprite) in sprites.iter() {
            if let Some(sprite) = sprite {
                unsafe {
                    backend.device.destroy_image_view(*sprite.texture, None);
                    backend.device.destroy_image(sprite.image, None);
                    backend.device.free_memory(sprite.memory, None);
                }
            }
        }
    }
}

impl Drop for SpriteManagerBackend {
    fn drop(&mut self) {
        // Signal thread to shut down by dropping sender
        drop(self.sender.clone());
        if let Some(thread) = self.command_thread.take() {
            if let Err(e) = thread.join() {
                log::error!("Failed to join command thread: {:?}", e);
            }
        }
    }
}

impl SpriteManager {
    pub fn new(sender: Sender<SpriteManagerCommand>) -> Self {
        Self { sender }
    }
}

impl zurie_render_glue::SpriteManager for SpriteManager {
    fn load_sprite(&self, info: ZurieLoadSpriteInfo) -> SpriteHandle {
        let (tx, rx) = unbounded();
        let load_info = match info {
            ZurieLoadSpriteInfo::Path(path) => LoadSpriteInfo::Path(path),
            ZurieLoadSpriteInfo::Buffer(buf) => LoadSpriteInfo::Buffer(buf),
        };
        self.sender
            .send(SpriteManagerCommand::LoadSprite(load_info, tx))
            .expect("Failed to send sprite load command");
        rx.recv().expect("Failed to receive sprite handle")
    }
}

impl Sprite {
    pub fn from_buffer(buffer: &[u8], backend: &Backend) -> anyhow::Result<Self> {
        let ase = AsepriteFile::read(buffer)?;
        let (texture, image, memory, width, height) = texture_from_ase(&ase, backend)?;
        Ok(Self {
            texture: Arc::new(texture),
            image,
            memory,
            width,
            height,
            path: None,
        })
    }

    pub fn from_file(path: &Path, backend: &Backend) -> anyhow::Result<Self> {
        info!("Loading sprite from {:?}", path);
        let ase = AsepriteFile::read_file(path)?;
        let path_str = path
            .to_str()
            .ok_or_else(|| anyhow::anyhow!("Invalid path"))?
            .to_string();
        let (texture, image, memory, width, height) = texture_from_ase(&ase, backend)?;
        info!("Sprite loaded");
        Ok(Self {
            texture: Arc::new(texture),
            image,
            memory,
            width,
            height,
            path: Some(path_str),
        })
    }

    pub fn create_error_sprite(backend: &Backend) -> anyhow::Result<Self> {
        let pixel_data = [255u8, 255, 255, 255]; // White pixel
        let (texture, image, memory, width, height) = create_texture(&pixel_data, 1, 1, backend)?;
        Ok(Self {
            texture: Arc::new(texture),
            image,
            memory,
            width,
            height,
            path: None,
        })
    }

    pub fn texture(&self) -> Arc<vk::ImageView> {
        self.texture.clone()
    }
}

fn create_texture(
    rgba_data: &[u8],
    width: u32,
    height: u32,
    backend: &Backend,
) -> anyhow::Result<(vk::ImageView, vk::Image, vk::DeviceMemory, u32, u32)> {
    let device = &backend.device;
    let command_pool = backend.command_pool;
    let queue = backend.graphics_queue;

    // Create staging buffer
    let buffer_size = (width * height * 4) as vk::DeviceSize;
    let staging_buffer_create_info = vk::BufferCreateInfo {
        size: buffer_size,
        usage: vk::BufferUsageFlags::TRANSFER_SRC,
        sharing_mode: vk::SharingMode::EXCLUSIVE,
        ..Default::default()
    };

    let staging_buffer = unsafe { device.create_buffer(&staging_buffer_create_info, None)? };
    let mem_requirements = unsafe { device.get_buffer_memory_requirements(staging_buffer) };

    let memory_type_index = find_memory_type(
        &backend,
        mem_requirements.memory_type_bits,
        vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
    )?;

    let memory_allocate_info = vk::MemoryAllocateInfo {
        allocation_size: mem_requirements.size,
        memory_type_index,
        ..Default::default()
    };

    let buffer_memory = unsafe { device.allocate_memory(&memory_allocate_info, None)? };

    unsafe {
        device.bind_buffer_memory(staging_buffer, buffer_memory, 0)?;
        let data_ptr =
            device.map_memory(buffer_memory, 0, buffer_size, vk::MemoryMapFlags::empty())?
                as *mut u8;
        data_ptr.copy_from_nonoverlapping(rgba_data.as_ptr(), rgba_data.len());
        device.unmap_memory(buffer_memory);
    }

    // Create image
    let image_create_info = vk::ImageCreateInfo {
        image_type: vk::ImageType::TYPE_2D,
        format: vk::Format::R8G8B8A8_SRGB,
        extent: vk::Extent3D {
            width,
            height,
            depth: 1,
        },
        mip_levels: 1,
        array_layers: 1,
        samples: vk::SampleCountFlags::TYPE_1,
        tiling: vk::ImageTiling::OPTIMAL,
        usage: vk::ImageUsageFlags::TRANSFER_DST | vk::ImageUsageFlags::SAMPLED,
        sharing_mode: vk::SharingMode::EXCLUSIVE,
        initial_layout: vk::ImageLayout::UNDEFINED,
        ..Default::default()
    };

    let image = unsafe { device.create_image(&image_create_info, None)? };
    let mem_requirements = unsafe { device.get_image_memory_requirements(image) };

    let memory_type_index = find_memory_type(
        &backend,
        mem_requirements.memory_type_bits,
        vk::MemoryPropertyFlags::DEVICE_LOCAL,
    )?;

    let memory_allocate_info = vk::MemoryAllocateInfo {
        allocation_size: mem_requirements.size,
        memory_type_index,
        ..Default::default()
    };

    let image_memory = unsafe { device.allocate_memory(&memory_allocate_info, None)? };
    unsafe { device.bind_image_memory(image, image_memory, 0)? };

    // Create command buffer
    let command_buffer = begin_single_time_commands(device, command_pool);

    // Transition image layout
    let barrier = vk::ImageMemoryBarrier {
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
    }

    // Copy buffer to image
    let buffer_image_copy = vk::BufferImageCopy {
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
            width,
            height,
            depth: 1,
        },
    };

    unsafe {
        device.cmd_copy_buffer_to_image(
            command_buffer,
            staging_buffer,
            image,
            vk::ImageLayout::TRANSFER_DST_OPTIMAL,
            &[buffer_image_copy],
        );
    }

    // Transition to shader readable
    let barrier = vk::ImageMemoryBarrier {
        src_access_mask: vk::AccessFlags::TRANSFER_WRITE,
        dst_access_mask: vk::AccessFlags::SHADER_READ,
        old_layout: vk::ImageLayout::TRANSFER_DST_OPTIMAL,
        new_layout: vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
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
            vk::PipelineStageFlags::TRANSFER,
            vk::PipelineStageFlags::FRAGMENT_SHADER,
            vk::DependencyFlags::empty(),
            &[],
            &[],
            &[barrier],
        );
        device.end_command_buffer(command_buffer)?;
    }

    // Submit command buffer
    end_single_time_commands(device, command_pool, queue, command_buffer);

    // Create image view
    let view_create_info = vk::ImageViewCreateInfo {
        image,
        view_type: vk::ImageViewType::TYPE_2D,
        format: vk::Format::R8G8B8A8_SRGB,
        components: vk::ComponentMapping {
            r: vk::ComponentSwizzle::IDENTITY,
            g: vk::ComponentSwizzle::IDENTITY,
            b: vk::ComponentSwizzle::IDENTITY,
            a: vk::ComponentSwizzle::IDENTITY,
        },
        subresource_range: vk::ImageSubresourceRange {
            aspect_mask: vk::ImageAspectFlags::COLOR,
            base_mip_level: 0,
            level_count: 1,
            base_array_layer: 0,
            layer_count: 1,
        },
        ..Default::default()
    };

    let image_view = unsafe { device.create_image_view(&view_create_info, None)? };

    // Cleanup staging buffer
    unsafe {
        device.destroy_buffer(staging_buffer, None);
        device.free_memory(buffer_memory, None);
    }

    Ok((image_view, image, image_memory, width, height))
}

fn texture_from_ase(
    ase: &AsepriteFile,
    backend: &Backend,
) -> anyhow::Result<(vk::ImageView, vk::Image, vk::DeviceMemory, u32, u32)> {
    let frame = ase.frame(0).image();
    let width = frame.width();
    let height = frame.height();
    let rgba_data: Vec<u8> = frame
        .pixels()
        .flat_map(|p| [p[0], p[1], p[2], p[3]])
        .collect();

    create_texture(&rgba_data, width, height, backend)
}

fn find_memory_type(
    backend: &Backend,
    type_filter: u32,
    properties: vk::MemoryPropertyFlags,
) -> anyhow::Result<u32> {
    let mem_properties = unsafe {
        backend
            .instance
            .get_physical_device_memory_properties(backend.physical_device)
    };
    for i in 0..mem_properties.memory_type_count {
        if (type_filter & (1 << i)) != 0
            && (mem_properties.memory_types[i as usize].property_flags & properties) == properties
        {
            return Ok(i);
        }
    }
    Err(anyhow::anyhow!("Failed to find suitable memory type"))
}
