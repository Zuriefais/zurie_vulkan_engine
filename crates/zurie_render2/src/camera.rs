use ash::vk;

use bytemuck::{Pod, Zeroable};
use glam::{Mat4, Vec2};

#[repr(C, packed)]
#[derive(Clone, Copy, Pod, Zeroable, Default)]
pub struct Camera {
    pub proj_mat: Mat4,
    pub cam_pos: Vec2,
}

pub fn create_uniform_buffer(
    instance: &ash::Instance,
    device: &ash::Device,
    physical_device: vk::PhysicalDevice,
    queue_family_indices: &Vec<u32>,
) -> (vk::Buffer, vk::DeviceMemory) {
    let buffer_size = std::mem::size_of::<Camera>() as vk::DeviceSize;

    // Define buffer creation info for uniform buffer
    let buffer_info = vk::BufferCreateInfo {
        s_type: vk::StructureType::BUFFER_CREATE_INFO,
        p_next: std::ptr::null(),
        flags: vk::BufferCreateFlags::empty(),
        size: buffer_size,
        usage: vk::BufferUsageFlags::UNIFORM_BUFFER, // Changed to UNIFORM_BUFFER
        sharing_mode: vk::SharingMode::EXCLUSIVE,
        queue_family_index_count: 1,
        p_queue_family_indices: queue_family_indices.as_ptr(),
        _marker: std::marker::PhantomData,
    };

    // Create the buffer
    let uniform_buffer = unsafe { device.create_buffer(&buffer_info, None) }
        .expect("Failed to create uniform buffer");

    // Get memory requirements
    let mem_requirements = unsafe { device.get_buffer_memory_requirements(uniform_buffer) };

    // Allocate memory (assuming allocate_buffer_memory helper function exists)
    let memory = crate::utils::allocate_buffer_memory(
        instance,
        device,
        physical_device,
        &mem_requirements,
        vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
    );

    // Bind memory to buffer
    unsafe {
        device
            .bind_buffer_memory(uniform_buffer, memory, 0)
            .expect("Failed to bind uniform buffer memory");
    }

    (uniform_buffer, memory)
}
fn allocate_buffer_memory(
    instance: &ash::Instance,
    device: &ash::Device,
    physical_device: vk::PhysicalDevice,
    mem_requirements: &vk::MemoryRequirements,
) -> vk::DeviceMemory {
    let mem_properties = unsafe { instance.get_physical_device_memory_properties(physical_device) };

    let memory_type_index = mem_properties
        .memory_types
        .iter()
        .enumerate()
        .find(|(i, mem_type)| {
            let type_filter = mem_requirements.memory_type_bits & (1 << i);
            let properties =
                vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT;
            type_filter != 0 && mem_type.property_flags.contains(properties)
        })
        .map(|(i, _)| i as u32)
        .expect("Failed to find suitable memory type");

    let alloc_info = vk::MemoryAllocateInfo {
        s_type: vk::StructureType::MEMORY_ALLOCATE_INFO,
        p_next: std::ptr::null(),
        allocation_size: mem_requirements.size,
        memory_type_index,
        _marker: std::marker::PhantomData,
    };

    unsafe { device.allocate_memory(&alloc_info, None) }.expect("Failed to allocate buffer memory")
}

fn upload_camera_data(device: &ash::Device, buffer_memory: vk::DeviceMemory, camera: Camera) {
    let data_ptr = unsafe {
        device
            .map_memory(
                buffer_memory,
                0,
                vk::WHOLE_SIZE,
                vk::MemoryMapFlags::empty(),
            )
            .expect("Failed to map memory") as *mut u8
    };

    let binding = [camera.clone()];
    let camera_bytes = bytemuck::cast_slice(&binding);

    unsafe {
        std::ptr::copy_nonoverlapping(camera_bytes.as_ptr(), data_ptr, camera_bytes.len());
        device.unmap_memory(buffer_memory);
    }
}

pub fn create_camera_buffer(
    instance: &ash::Instance,
    device: &ash::Device,
    physical_device: vk::PhysicalDevice,
    queue_family_indeces: &Vec<u32>,
    camera: Camera,
) -> vk::Buffer {
    let (camera_buffer, device_memory) =
        create_uniform_buffer(instance, device, physical_device, queue_family_indeces);

    upload_camera_data(device, device_memory, camera);

    camera_buffer
}
