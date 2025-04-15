use ash::vk;
use naga::back::spv; // For generating SPIR-V
use naga::front::wgsl; // For parsing WGSL
use naga::valid::{Capabilities, ValidationFlags, Validator};
use std::ptr;

pub fn create_shader_module(device: &ash::Device, wgsl_source: &str) -> vk::ShaderModule {
    // Step 1: Parse WGSL source into a Naga module
    let module = wgsl::parse_str(wgsl_source).expect("Failed to parse WGSL source");

    // Step 2: Validate the module (recommended for Vulkan compatibility)
    let mut validator = Validator::new(ValidationFlags::all(), Capabilities::all());
    let module_info = validator.validate(&module).expect("WGSL validation failed");

    // Step 3: Convert the module to SPIR-V
    let spv_options = spv::Options {
        flags: spv::WriterFlags::empty(), // Customize if needed (e.g., DEBUG)
        ..Default::default()
    };
    let spv_binary = spv::write_vec(&module, &module_info, &spv_options, None)
        .expect("Failed to convert WGSL to SPIR-V");

    // Step 4: Create the Vulkan shader module with the SPIR-V binary
    let shader_module_create_info = vk::ShaderModuleCreateInfo {
        s_type: vk::StructureType::SHADER_MODULE_CREATE_INFO,
        p_next: ptr::null(),
        flags: vk::ShaderModuleCreateFlags::empty(),
        code_size: spv_binary.len() * std::mem::size_of::<u32>(), // Size in bytes
        p_code: spv_binary.as_ptr(), // Pointer to SPIR-V binary (u32 words)
        _marker: std::marker::PhantomData,
    };

    unsafe {
        device
            .create_shader_module(&shader_module_create_info, None)
            .expect("Failed to create Shader Module!")
    }
}
