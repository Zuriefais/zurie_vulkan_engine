use crate::resources::resource_manager::ResourceManager;
use crate::sprite_manager;
use crate::vertex::{InstanceData, create_instance_buffer};
use ash::vk;
use zurie_render_glue::FrameContext;

use super::{
    backend::Backend,
    pass::{egui_pass::EGUIPass, obj_pass::ObjPass},
};

pub struct Renderer {
    pub backend: Backend,
    pub sprite_manager_backend: sprite_manager::SpriteManagerBackend,
    pub resource_manager: ResourceManager,
    pub obj_pass: ObjPass,
    pub egui_pass: EGUIPass,
    pub current_frame: usize,
}

impl zurie_render_glue::RenderBackend for Renderer {
    fn init(config: zurie_render_glue::RenderConfig) -> Result<Self, anyhow::Error> {
        let backend = Backend::new(config.window.clone())?;
        let resource_manager = ResourceManager::new(&backend)?;
        let mut obj_pass = ObjPass::new(&backend, &resource_manager)?;
        let mut egui_pass = EGUIPass::new(&backend, config.egui_context)?;
        obj_pass.init(&backend, backend.swapchain_extent())?;
        egui_pass.init(&backend, backend.swapchain_extent())?;
        let sprite_manager_backend = sprite_manager::SpriteManagerBackend::new(&backend)?;
        Ok(Self {
            backend,
            resource_manager,
            obj_pass,
            egui_pass,
            current_frame: 0,
            sprite_manager_backend,
        })
    }

    fn render<I>(&mut self, frame_context: &FrameContext, objects: I) -> anyhow::Result<()>
    where
        I: Iterator<Item = zurie_types::Object>,
    {
        let wait_fences = [self.backend.in_flight_fences()[self.current_frame]];
        unsafe {
            self.backend
                .device()
                .wait_for_fences(&wait_fences, true, u64::MAX)?;
            let (image_index, _) = self.backend.acquire_next_image(self.current_frame)?;

            let mut objects = objects.collect::<Vec<_>>();
            objects.sort_by_key(|data| data.z_index);
            // Update instance buffer
            let instance_data: Vec<InstanceData> = objects
                .iter()
                .map(|obj| InstanceData {
                    position: obj.position,
                    scale: obj.scale,
                    color: obj.color,
                })
                .collect::<Vec<InstanceData>>();

            if !instance_data.is_empty() {
                self.backend
                    .device()
                    .destroy_buffer(self.obj_pass.instance_buffer, None);
                self.backend
                    .device()
                    .free_memory(self.obj_pass.instance_buffer_memory, None);

                let (instance_buffer, instance_buffer_memory) = create_instance_buffer(
                    &self.backend.instance,
                    &self.backend.device,
                    self.backend.physical_device,
                    &self.backend.queue_family_indices,
                    &instance_data,
                );
                self.obj_pass.instance_buffer = instance_buffer;
                self.obj_pass.instance_buffer_memory = instance_buffer_memory;
            }

            let command_buffer = self.backend.command_buffers()[image_index as usize];
            self.backend.device().reset_command_pool(
                self.backend.command_pool(),
                vk::CommandPoolResetFlags::empty(),
            )?;

            let begin_info = vk::CommandBufferBeginInfo::default()
                .flags(vk::CommandBufferUsageFlags::SIMULTANEOUS_USE);
            self.backend
                .device()
                .begin_command_buffer(command_buffer, &begin_info)?;

            self.obj_pass.record(
                command_buffer,
                image_index as usize,
                &self.backend,
                &self.resource_manager,
                &frame_context,
                instance_data,
            )?;
            self.egui_pass.record(
                command_buffer,
                &self.backend,
                &self.resource_manager,
                &frame_context,
            )?;

            self.backend.device().end_command_buffer(command_buffer)?;

            self.backend
                .submit_and_present(image_index, self.current_frame, command_buffer)?;
        }

        self.current_frame = (self.current_frame + 1) % crate::constants::MAX_FRAMES_IN_FLIGHT;
        Ok(())
    }

    fn handle_window_event(&mut self, event: &winit::event::WindowEvent) -> anyhow::Result<()> {
        Ok(())
    }

    fn resize_window(
        &mut self,
        size: (u32, u32),
        frame_context: &FrameContext,
    ) -> anyhow::Result<()> {
        self.backend.recreate_swapchain(size)?;
        let new_extent = self.backend.swapchain_extent();
        self.egui_pass.resize(&self.backend, new_extent)?;
        self.obj_pass.resize(&self.backend, new_extent)?;
        Ok(())
    }

    fn get_sprite_manager(&self) -> Box<dyn zurie_render_glue::SpriteManager> {
        let sprite_manager = Box::new(self.sprite_manager_backend.get_manager());
        sprite_manager
    }
}
