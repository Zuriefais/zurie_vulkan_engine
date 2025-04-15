#[derive(Pod, Zeroable, Clone, Copy)]
#[repr(C)]
pub struct TriangleVertex {
    pub vert_position: Vec2,
}

/// The vertex type that describes the unique data per instance.
#[derive(Pod, Zeroable, Clone, Copy, Default)]
#[repr(C)]
pub struct InstanceData {
    pub position: Vec2,
    pub scale: Vec2,
    pub color: Vec4,
}
pub const QUAD: [TriangleVertex; 4] = [
    TriangleVertex {
        vert_position: Vec2::new(-0.5, -0.5),
    }, // Bottom-left
    TriangleVertex {
        vert_position: Vec2::new(0.5, -0.5),
    }, // Bottom-right
    TriangleVertex {
        vert_position: Vec2::new(-0.5, 0.5),
    }, // Top-left
    TriangleVertex {
        vert_position: Vec2::new(0.5, 0.5),
    }, // Top-right
];

pub const TRIANGLE: [TriangleVertex; 3] = [
    TriangleVertex {
        vert_position: Vec2::new(0.0, -0.5),
    },
    TriangleVertex {
        vert_position: Vec2::new(-0.5, 0.5),
    },
    TriangleVertex {
        vert_position: Vec2::new(0.5, 0.5),
    },
];

use ash::vk;
use bytemuck::{Pod, Zeroable};
use glam::{Vec2, Vec4};

pub fn create_vertex_buffer(
    instance: &ash::Instance,
    device: &ash::Device,
    physical_device: vk::PhysicalDevice,
    queue_family_indices: &Vec<u32>,
) -> (vk::Buffer, vk::DeviceMemory) {
    let buffer_size = std::mem::size_of_val(&QUAD) as vk::DeviceSize;

    // Define buffer creation info
    let buffer_info = vk::BufferCreateInfo {
        s_type: vk::StructureType::BUFFER_CREATE_INFO,
        p_next: std::ptr::null(),
        flags: vk::BufferCreateFlags::empty(),
        size: buffer_size,
        usage: vk::BufferUsageFlags::VERTEX_BUFFER,
        sharing_mode: vk::SharingMode::EXCLUSIVE,
        queue_family_index_count: 1,
        p_queue_family_indices: queue_family_indices.as_ptr(),
        _marker: std::marker::PhantomData,
    };

    // Create the buffer
    let vertex_buffer = unsafe { device.create_buffer(&buffer_info, None) }
        .expect("Failed to create vertex buffer");

    // Get memory requirements
    let mem_requirements = unsafe { device.get_buffer_memory_requirements(vertex_buffer) };

    // Allocate memory (helper function below)
    let memory = allocate_buffer_memory(instance, device, physical_device, &mem_requirements);

    // Bind memory to buffer
    unsafe {
        device
            .bind_buffer_memory(vertex_buffer, memory, 0)
            .expect("Failed to bind buffer memory");
    }

    (vertex_buffer, memory)
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

fn upload_vertex_data(device: &ash::Device, buffer_memory: vk::DeviceMemory) {
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

    let vertex_bytes = bytemuck::cast_slice(&QUAD);

    unsafe {
        std::ptr::copy_nonoverlapping(vertex_bytes.as_ptr(), data_ptr, vertex_bytes.len());
        device.unmap_memory(buffer_memory);
    }
}

pub fn create_quad_buffer(
    instance: &ash::Instance,
    device: &ash::Device,
    physical_device: vk::PhysicalDevice,
    queue_family_indices: &Vec<u32>,
) -> (vk::Buffer, vk::DeviceMemory) {
    let (quad_buffer, device_memory) =
        create_vertex_buffer(instance, device, physical_device, queue_family_indices);
    let data_ptr = unsafe {
        device
            .map_memory(
                device_memory,
                0,
                vk::WHOLE_SIZE,
                vk::MemoryMapFlags::empty(),
            )
            .expect("Failed to map memory") as *mut u8
    };
    let vertex_bytes = bytemuck::cast_slice(&QUAD);
    unsafe {
        std::ptr::copy_nonoverlapping(vertex_bytes.as_ptr(), data_ptr, vertex_bytes.len());
        device.unmap_memory(device_memory);
    }
    (quad_buffer, device_memory)
}

pub fn create_instance_buffer(
    instance: &ash::Instance,
    device: &ash::Device,
    physical_device: vk::PhysicalDevice,
    queue_family_indices: &Vec<u32>,
    instance_data: &[InstanceData],
) -> (vk::Buffer, vk::DeviceMemory) {
    let buffer_size = std::mem::size_of_val(instance_data) as vk::DeviceSize;

    let buffer_info = vk::BufferCreateInfo {
        s_type: vk::StructureType::BUFFER_CREATE_INFO,
        p_next: std::ptr::null(),
        flags: vk::BufferCreateFlags::empty(),
        size: buffer_size,
        usage: vk::BufferUsageFlags::VERTEX_BUFFER,
        sharing_mode: vk::SharingMode::EXCLUSIVE,
        queue_family_index_count: queue_family_indices.len() as u32,
        p_queue_family_indices: queue_family_indices.as_ptr(),
        _marker: std::marker::PhantomData,
    };

    let instance_buffer = unsafe { device.create_buffer(&buffer_info, None) }
        .expect("Failed to create instance buffer");

    let mem_requirements = unsafe { device.get_buffer_memory_requirements(instance_buffer) };
    let memory = allocate_buffer_memory(instance, device, physical_device, &mem_requirements);

    unsafe {
        device
            .bind_buffer_memory(instance_buffer, memory, 0)
            .expect("Failed to bind instance buffer memory");

        let data_ptr = device
            .map_memory(memory, 0, vk::WHOLE_SIZE, vk::MemoryMapFlags::empty())
            .expect("Failed to map instance buffer memory") as *mut u8;

        let bytes = bytemuck::cast_slice(instance_data);
        std::ptr::copy_nonoverlapping(bytes.as_ptr(), data_ptr, bytes.len());
        device.unmap_memory(memory);
    }

    (instance_buffer, memory)
}
