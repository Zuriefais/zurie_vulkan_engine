use ash::vk;

use crate::render::backend::Backend;

pub struct ResourceManager {
    // Track buffers, textures, etc.
    quad_buffer: vk::Buffer,
    quad_buffer_memory: vk::DeviceMemory,
    // Add texture pool, material pool, etc.
}

impl ResourceManager {
    pub fn new(backend: &Backend) -> anyhow::Result<Self> {
        let (quad_buffer, quad_buffer_memory) = crate::vertex::create_quad_buffer(
            &backend.instance,
            &backend.device,
            backend.physical_device,
            &backend.queue_family_indices,
        );
        Ok(Self {
            quad_buffer,
            quad_buffer_memory,
        })
    }

    pub fn quad_buffer(&self) -> vk::Buffer {
        self.quad_buffer
    }
}
