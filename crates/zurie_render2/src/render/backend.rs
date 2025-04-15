use crate::constants::*;
use crate::debug::setup_debug_utils;
use crate::platforms;
use crate::structures::SurfaceStuff;
use crate::structures::SyncObjects;
use crate::utils::swapchain::*;
use crate::utils::vulkan_init::*;
use ash::vk;
use ash::vk::Handle;
use std::ptr;
use std::sync::Arc;
use winit::window::Window;

pub struct Backend {
    pub entry: ash::Entry,
    pub instance: ash::Instance,
    pub device: ash::Device,
    pub physical_device: vk::PhysicalDevice,
    pub graphics_queue: vk::Queue,
    pub present_queue: vk::Queue,
    pub swapchain_loader: ash::khr::swapchain::Device,
    pub swapchain: vk::SwapchainKHR,
    pub swapchain_images: Vec<vk::Image>,
    pub swapchain_imageviews: Vec<vk::ImageView>,
    pub swapchain_extent: vk::Extent2D,
    pub swapchain_format: vk::Format,
    pub surface_loader: ash::khr::surface::Instance,
    pub surface: vk::SurfaceKHR,
    pub command_pool: vk::CommandPool,
    pub command_buffers: Vec<vk::CommandBuffer>,
    pub image_available_semaphores: Vec<vk::Semaphore>,
    pub render_finished_semaphores: Vec<vk::Semaphore>,
    pub in_flight_fences: Vec<vk::Fence>,
    pub queue_family_indices: Vec<u32>,
    debug_utils_loader: ash::ext::debug_utils::Instance,
    debug_messenger: vk::DebugUtilsMessengerEXT,
    window: Arc<Window>,
}

impl Backend {
    pub fn new(window: Arc<Window>) -> anyhow::Result<Self> {
        let entry = unsafe { ash::Entry::load().unwrap() };
        let instance = create_instance(
            &entry,
            WINDOW_TITLE,
            VALIDATION.is_enable,
            &VALIDATION.required_validation_layers.to_vec(),
        )?;
        let surface_stuff = create_surface(
            &entry,
            &instance,
            window.clone(),
            WINDOW_WIDTH,
            WINDOW_HEIGHT,
        );
        let (debug_utils_loader, debug_messenger) =
            setup_debug_utils(VALIDATION.is_enable, &entry, &instance);
        let physical_device = pick_physical_device(&instance, &surface_stuff)?;
        let (device, family_indices) =
            create_logical_device(&instance, physical_device, &surface_stuff);
        let graphics_queue =
            unsafe { device.get_device_queue(family_indices.graphics_family.unwrap(), 0) };
        let present_queue =
            unsafe { device.get_device_queue(family_indices.present_family.unwrap(), 0) };
        let swapchain_stuff = create_swapchain(
            &instance,
            &device,
            physical_device,
            &window,
            &surface_stuff,
            &family_indices,
        );
        let swapchain_imageviews = create_image_views(
            &device,
            swapchain_stuff.swapchain_format,
            &swapchain_stuff.swapchain_images,
        );
        let command_pool = create_command_pool(&device, &family_indices);
        let command_buffers =
            create_command_buffers_dynamic(&device, command_pool, &swapchain_imageviews);
        let sync_objects = create_sync_objects(&device);

        let queue_family_indices =
            if family_indices.graphics_family == family_indices.present_family {
                vec![family_indices.graphics_family.unwrap()]
            } else {
                vec![
                    family_indices.graphics_family.unwrap(),
                    family_indices.present_family.unwrap(),
                ]
            };

        Ok(Self {
            entry,
            instance,
            device,
            physical_device,
            graphics_queue,
            present_queue,
            swapchain_loader: swapchain_stuff.swapchain_loader,
            swapchain: swapchain_stuff.swapchain,
            swapchain_images: swapchain_stuff.swapchain_images,
            swapchain_imageviews,
            swapchain_extent: swapchain_stuff.swapchain_extent,
            swapchain_format: swapchain_stuff.swapchain_format,
            surface_loader: surface_stuff.surface_loader,
            surface: surface_stuff.surface,
            command_pool,
            command_buffers,
            image_available_semaphores: sync_objects.image_available_semaphores,
            render_finished_semaphores: sync_objects.render_finished_semaphores,
            in_flight_fences: sync_objects.inflight_fences,
            queue_family_indices,
            debug_utils_loader,
            debug_messenger,
            window,
        })
    }

    pub fn device(&self) -> &ash::Device {
        &self.device
    }
    pub fn swapchain_extent(&self) -> vk::Extent2D {
        self.swapchain_extent
    }
    pub fn command_pool(&self) -> vk::CommandPool {
        self.command_pool
    }
    pub fn command_buffers(&self) -> &[vk::CommandBuffer] {
        &self.command_buffers
    }
    pub fn in_flight_fences(&self) -> &[vk::Fence] {
        &self.in_flight_fences
    }

    pub fn acquire_next_image(&self, frame: usize) -> anyhow::Result<(u32, bool)> {
        unsafe {
            self.swapchain_loader.acquire_next_image(
                self.swapchain,
                u64::MAX,
                self.image_available_semaphores[frame],
                vk::Fence::null(),
            )
        }
        .map_err(|e| anyhow::anyhow!("Failed to acquire next image: {}", e))
    }

    pub fn submit_and_present(
        &self,
        image_index: u32,
        frame: usize,
        command_buffer: vk::CommandBuffer,
    ) -> anyhow::Result<()> {
        let wait_semaphores = [self.image_available_semaphores[frame]];
        let wait_stages = [vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT];
        let signal_semaphores = [self.render_finished_semaphores[frame]];
        let command_buffers = [command_buffer];
        let submit_infos = [vk::SubmitInfo {
            s_type: vk::StructureType::SUBMIT_INFO,
            p_next: ptr::null(),
            wait_semaphore_count: wait_semaphores.len() as u32,
            p_wait_semaphores: wait_semaphores.as_ptr(),
            p_wait_dst_stage_mask: wait_stages.as_ptr(),
            command_buffer_count: 1,
            p_command_buffers: command_buffers.as_ptr(),
            signal_semaphore_count: signal_semaphores.len() as u32,
            p_signal_semaphores: signal_semaphores.as_ptr(),
            _marker: std::marker::PhantomData,
        }];

        unsafe {
            self.device
                .reset_fences(&[self.in_flight_fences[frame]])
                .expect("Failed to reset Fence!");
            self.device
                .queue_submit(
                    self.graphics_queue,
                    &submit_infos,
                    self.in_flight_fences[frame],
                )
                .expect("Failed to execute queue submit.");

            let swapchains = [self.swapchain];
            let present_info = vk::PresentInfoKHR {
                s_type: vk::StructureType::PRESENT_INFO_KHR,
                p_next: ptr::null(),
                wait_semaphore_count: 1,
                p_wait_semaphores: signal_semaphores.as_ptr(),
                swapchain_count: 1,
                p_swapchains: swapchains.as_ptr(),
                p_image_indices: &image_index,
                p_results: ptr::null_mut(),
                _marker: std::marker::PhantomData,
            };

            self.swapchain_loader
                .queue_present(self.present_queue, &present_info)
                .expect("Failed to execute queue present.");
        }

        Ok(())
    }

    pub fn recreate_swapchain(&mut self, new_size: (u32, u32)) -> anyhow::Result<()> {
        unsafe {
            self.device
                .device_wait_idle()
                .expect("Failed to wait for device idle");

            // Clean up existing swapchain resources
            if !self.command_buffers.is_empty() {
                self.device
                    .free_command_buffers(self.command_pool, &self.command_buffers);
                self.command_buffers.clear();
            }
            if !self.swapchain_imageviews.is_empty() {
                for &image_view in self.swapchain_imageviews.iter() {
                    if !image_view.is_null() {
                        self.device.destroy_image_view(image_view, None);
                    }
                }
                self.swapchain_imageviews.clear();
            }
            if !self.swapchain.is_null() {
                self.swapchain_loader
                    .destroy_swapchain(self.swapchain, None);
                self.swapchain = vk::SwapchainKHR::null();
            }

            // Recreate swapchain
            let surface_stuff = SurfaceStuff {
                surface_loader: self.surface_loader.clone(),
                surface: self.surface,
                screen_width: new_size.0,
                screen_height: new_size.1,
            };
            let queue_family =
                find_queue_family(&self.instance, self.physical_device, &surface_stuff);
            let swapchain_stuff = create_swapchain(
                &self.instance,
                &self.device,
                self.physical_device,
                &self.window,
                &surface_stuff,
                &queue_family,
            );
            let swapchain_imageviews = create_image_views(
                &self.device,
                swapchain_stuff.swapchain_format,
                &swapchain_stuff.swapchain_images,
            );
            let command_buffers = create_command_buffers_dynamic(
                &self.device,
                self.command_pool,
                &swapchain_imageviews,
            );

            self.swapchain_loader = swapchain_stuff.swapchain_loader;
            self.swapchain = swapchain_stuff.swapchain;
            self.swapchain_format = swapchain_stuff.swapchain_format;
            self.swapchain_images = swapchain_stuff.swapchain_images;
            self.swapchain_extent = swapchain_stuff.swapchain_extent;
            self.swapchain_imageviews = swapchain_imageviews;
            self.command_buffers = command_buffers;
            self.queue_family_indices = swapchain_stuff.queue_family_indices;
        }
        Ok(())
    }
}

fn create_sync_objects(device: &ash::Device) -> SyncObjects {
    let mut sync_objects = SyncObjects {
        image_available_semaphores: vec![],
        render_finished_semaphores: vec![],
        inflight_fences: vec![],
    };
    let semaphore_create_info = vk::SemaphoreCreateInfo {
        s_type: vk::StructureType::SEMAPHORE_CREATE_INFO,
        p_next: ptr::null(),
        flags: vk::SemaphoreCreateFlags::empty(),
        _marker: std::marker::PhantomData,
    };
    let fence_create_info = vk::FenceCreateInfo {
        s_type: vk::StructureType::FENCE_CREATE_INFO,
        p_next: ptr::null(),
        flags: vk::FenceCreateFlags::SIGNALED,
        _marker: std::marker::PhantomData,
    };
    for _ in 0..MAX_FRAMES_IN_FLIGHT {
        unsafe {
            let image_available_semaphore = device
                .create_semaphore(&semaphore_create_info, None)
                .expect("Failed to create Semaphore Object!");
            let render_finished_semaphore = device
                .create_semaphore(&semaphore_create_info, None)
                .expect("Failed to create Semaphore Object!");
            let inflight_fence = device
                .create_fence(&fence_create_info, None)
                .expect("Failed to create Fence Object!");
            sync_objects
                .image_available_semaphores
                .push(image_available_semaphore);
            sync_objects
                .render_finished_semaphores
                .push(render_finished_semaphore);
            sync_objects.inflight_fences.push(inflight_fence);
        }
    }
    sync_objects
}

impl Drop for Backend {
    fn drop(&mut self) {
        unsafe {
            self.device
                .device_wait_idle()
                .expect("Failed to wait for device idle");

            for i in 0..MAX_FRAMES_IN_FLIGHT {
                self.device
                    .destroy_semaphore(self.image_available_semaphores[i], None);
                self.device
                    .destroy_semaphore(self.render_finished_semaphores[i], None);
                self.device.destroy_fence(self.in_flight_fences[i], None);
            }
            self.device.destroy_command_pool(self.command_pool, None);
            for &imageview in self.swapchain_imageviews.iter() {
                self.device.destroy_image_view(imageview, None);
            }
            self.swapchain_loader
                .destroy_swapchain(self.swapchain, None);
            self.device.destroy_device(None);
            self.surface_loader.destroy_surface(self.surface, None);
            if VALIDATION.is_enable {
                self.debug_utils_loader
                    .destroy_debug_utils_messenger(self.debug_messenger, None);
            }
            self.instance.destroy_instance(None);
        }
    }
}
