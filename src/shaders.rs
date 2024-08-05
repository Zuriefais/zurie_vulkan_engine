use std::{fs, sync::Arc};

use log::info;
use naga::ShaderStage;
use vulkano::{
    device::Device,
    shader::{ShaderModule, ShaderModuleCreateInfo},
};

pub fn wgsl_to_shader_module(
    path: String,
    device: Arc<Device>,
    entry_point: String,
    shader_stage: ShaderStage,
) -> Arc<ShaderModule> {
    let contents = fs::read_to_string(path).unwrap();

    let wgsl_module = naga::front::wgsl::parse_str(contents.as_str()).unwrap();
    let info = naga::valid::Validator::new(
        naga::valid::ValidationFlags::all(),
        naga::valid::Capabilities::all(),
    )
    .validate(&wgsl_module)
    .unwrap();
    let spirv_module = naga::back::spv::write_vec(
        &wgsl_module,
        &info,
        &naga::back::spv::Options::default(),
        Some(&naga::back::spv::PipelineOptions {
            shader_stage,
            entry_point,
        }),
    )
    .unwrap();
    info!("Spirv module compiled from wgsl: {:?}", spirv_module);
    unsafe { ShaderModule::new(device, ShaderModuleCreateInfo::new(&spirv_module)).unwrap() }
}
