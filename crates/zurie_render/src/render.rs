use std::{collections::HashMap, env, sync::Arc};

use log::info;
use vulkano::{
    Validated, VulkanError, VulkanLibrary,
    command_buffer::allocator::{
        StandardCommandBufferAllocator, StandardCommandBufferAllocatorCreateInfo,
    },
    descriptor_set::allocator::StandardDescriptorSetAllocator,
    device::{
        Device, DeviceCreateInfo, DeviceExtensions, Features, Queue, QueueCreateInfo, QueueFlags,
        physical::{PhysicalDevice, PhysicalDeviceType},
    },
    format::Format,
    image::{Image, ImageCreateInfo, ImageType, ImageUsage, view::ImageView},
    instance::{Instance, InstanceCreateFlags, InstanceCreateInfo},
    memory::allocator::{AllocationCreateInfo, StandardMemoryAllocator},
    swapchain::{self, PresentMode, Surface, Swapchain, SwapchainCreateInfo, SwapchainPresentInfo},
    sync::{self, GpuFuture},
};
use winit::window::Window;

pub struct Renderer {
    window: Arc<Window>,
    pub gfx_queue: Arc<Queue>,
    pub compute_queue: Arc<Queue>,
    swapchain: Arc<Swapchain>,
    final_views: Vec<Arc<ImageView>>,
    pub memory_allocator: Arc<StandardMemoryAllocator>,
    additional_image_views: HashMap<usize, Arc<ImageView>>,
    recreate_swapchain: bool,
    previous_frame_end: Option<Box<dyn GpuFuture>>,
    image_index: u32,
    present_mode: vulkano::swapchain::PresentMode,
    pub device: Arc<Device>,
    pub descriptor_set_allocator: Arc<StandardDescriptorSetAllocator>,
    pub command_buffer_allocator: Arc<StandardCommandBufferAllocator>,
    pub output_format: Format,
}

impl Renderer {
    pub fn new(window: Arc<winit::window::Window>) -> Renderer {
        let library = VulkanLibrary::new().expect("no local Vulkan library/DLL");
        let required_extensions = Surface::required_extensions(&window);
        let instance = Instance::new(library, InstanceCreateInfo {
            flags: InstanceCreateFlags::ENUMERATE_PORTABILITY,
            enabled_extensions: required_extensions,
            ..Default::default()
        })
        .expect("failed to create instance");
        let surface = loop {
            match Surface::from_window(instance.clone(), window.clone()) {
                Ok(surface) => break surface,
                Err(e) => {
                    log::warn!("Failed to create surface: {}", e);
                    std::thread::sleep(std::time::Duration::from_millis(100));
                }
            }
        };
        let device_extensions = DeviceExtensions {
            khr_swapchain: true,
            ..DeviceExtensions::empty()
        };

        let (physical_device, _) = instance
            .enumerate_physical_devices()
            .unwrap()
            .filter(|p| p.supported_extensions().contains(&device_extensions))
            .filter_map(|p| {
                p.queue_family_properties()
                    .iter()
                    .enumerate()
                    .position(|(i, q)| {
                        q.queue_flags.intersects(QueueFlags::GRAPHICS)
                            && p.surface_support(i as u32, &surface).unwrap_or(false)
                    })
                    .map(|i| (p, i as u32))
            })
            .min_by_key(|(p, _)| {
                // We assign a lower score to device types that are likely to be faster/better.
                match p.properties().device_type {
                    PhysicalDeviceType::DiscreteGpu => 0,
                    PhysicalDeviceType::IntegratedGpu => 1,
                    PhysicalDeviceType::VirtualGpu => 2,
                    PhysicalDeviceType::Cpu => 3,
                    PhysicalDeviceType::Other => 4,
                    _ => 5,
                }
            })
            .expect("no suitable physical device found");

        info!(
            "Using device: {} (type: {:?})",
            physical_device.properties().device_name,
            physical_device.properties().device_type,
        );

        let (device, gfx_queue, compute_queue) =
            Self::create_device(physical_device, device_extensions, Default::default());
        let present_mode = if env::var("DisableVsync").is_ok() {
            vulkano::swapchain::PresentMode::Mailbox
        } else {
            vulkano::swapchain::PresentMode::Fifo
        };

        let (swapchain, final_views, output_format) =
            Self::create_swapchain(device.clone(), &window, surface);
        let memory_allocator = Arc::new(StandardMemoryAllocator::new_default(device.clone()));
        let descriptor_set_allocator = Arc::new(StandardDescriptorSetAllocator::new(
            device.clone(),
            Default::default(),
        ));
        let previous_frame_end = Some(sync::now(device.clone()).boxed());
        let command_buffer_allocator = Arc::new(StandardCommandBufferAllocator::new(
            device.clone(),
            StandardCommandBufferAllocatorCreateInfo {
                secondary_buffer_count: 32,
                ..Default::default()
            },
        ));

        Renderer {
            window,
            gfx_queue,
            compute_queue,
            swapchain,
            final_views,
            memory_allocator,
            additional_image_views: HashMap::default(),
            recreate_swapchain: false,
            previous_frame_end,
            image_index: 0,
            present_mode,
            device,
            descriptor_set_allocator,
            command_buffer_allocator,
            output_format,
        }
    }

    fn create_device(
        physical_device: Arc<PhysicalDevice>,
        device_extensions: DeviceExtensions,
        features: Features,
    ) -> (Arc<Device>, Arc<Queue>, Arc<Queue>) {
        let queue_family_graphics = physical_device
            .queue_family_properties()
            .iter()
            .enumerate()
            .map(|(i, q)| (i as u32, q))
            .find(|(_i, q)| q.queue_flags.intersects(QueueFlags::GRAPHICS))
            .map(|(i, _)| i)
            .expect("could not find a queue that supports graphics");
        // Try finding a separate queue for compute
        let queue_family_compute = physical_device
            .queue_family_properties()
            .iter()
            .enumerate()
            .map(|(i, q)| (i as u32, q))
            .find(|(i, q)| {
                q.queue_flags.intersects(QueueFlags::COMPUTE) && *i != queue_family_graphics
            })
            .map(|(i, _)| i);
        let is_separate_compute_queue = false;

        let queue_create_infos = if let Some(queue_family_compute) = queue_family_compute {
            vec![
                QueueCreateInfo {
                    queue_family_index: queue_family_graphics,
                    ..Default::default()
                },
                QueueCreateInfo {
                    queue_family_index: queue_family_compute,
                    ..Default::default()
                },
            ]
        } else {
            vec![QueueCreateInfo {
                queue_family_index: queue_family_graphics,
                ..Default::default()
            }]
        };

        let (device, mut queues) = {
            Device::new(physical_device, DeviceCreateInfo {
                queue_create_infos,
                enabled_extensions: device_extensions,
                enabled_features: features,
                ..Default::default()
            })
            .expect("failed to create device")
        };
        let gfx_queue = queues.next().unwrap();
        let compute_queue = if is_separate_compute_queue {
            queues.next().unwrap()
        } else {
            gfx_queue.clone()
        };
        (device, gfx_queue, compute_queue)
    }

    fn create_swapchain(
        device: Arc<Device>,
        window: &Arc<Window>,
        surface: Arc<Surface>,
    ) -> (Arc<Swapchain>, Vec<Arc<ImageView>>, Format) {
        info!(
            "Available formats: {:?}",
            device
                .physical_device()
                .surface_formats(&surface, Default::default())
                .unwrap()
        );
        let surface_capabilities = device
            .physical_device()
            .surface_capabilities(&surface, Default::default())
            .unwrap();

        let image_format = if env::var("WAYLAND_DISPLAY").is_ok() {
            Format::B8G8R8A8_UNORM
        } else {
            device
                .physical_device()
                .surface_formats(&surface, Default::default())
                .unwrap()[0]
                .0
        };

        let (swapchain, images) = Swapchain::new(device, surface, {
            let mut create_info = SwapchainCreateInfo {
                min_image_count: surface_capabilities.min_image_count.max(2),
                image_format,
                image_extent: window.inner_size().into(),
                image_usage: ImageUsage::COLOR_ATTACHMENT,
                composite_alpha: surface_capabilities
                    .supported_composite_alpha
                    .into_iter()
                    .next()
                    .unwrap(),
                ..Default::default()
            };
            // Get present mode from window descriptor
            create_info.present_mode = Self::create_swapchain_present_mode();
            create_info
        })
        .unwrap();
        let images = images
            .into_iter()
            .map(|image| ImageView::new_default(image).unwrap())
            .collect::<Vec<_>>();

        (swapchain, images, image_format)
    }

    fn create_swapchain_present_mode() -> PresentMode {
        if env::var("DisableVsync").is_ok() {
            vulkano::swapchain::PresentMode::Mailbox
        } else {
            vulkano::swapchain::PresentMode::Fifo
        }
    }

    pub fn swapchain_format(&self) -> Format {
        self.final_views[self.image_index as usize].format()
    }

    pub fn image_index(&self) -> u32 {
        self.image_index
    }

    pub fn gfx_queue(&self) -> Arc<Queue> {
        self.gfx_queue.clone()
    }

    pub fn compute_queue(&self) -> Arc<Queue> {
        self.compute_queue.clone()
    }

    pub fn surface(&self) -> Arc<Surface> {
        self.swapchain.surface().clone()
    }

    pub fn window(&self) -> &Window {
        &self.window
    }

    pub fn window_size(&self) -> [u32; 2] {
        let size = self.window().inner_size();
        [size.width, size.height]
    }

    pub fn swapchain_image_size(&self) -> [u32; 2] {
        self.final_views[0].image().extent()[0..2]
            .try_into()
            .unwrap()
    }

    pub fn swapchain_image_view(&self) -> Arc<ImageView> {
        self.final_views[self.image_index as usize].clone()
    }

    pub fn resolution(&self) -> [f32; 2] {
        let size = self.window().inner_size();
        let scale_factor = self.window().scale_factor();
        [
            (size.width as f64 / scale_factor) as f32,
            (size.height as f64 / scale_factor) as f32,
        ]
    }

    pub fn aspect_ratio(&self) -> f32 {
        let dims = self.window_size();
        dims[0] as f32 / dims[1] as f32
    }

    pub fn resize(&mut self) {
        self.recreate_swapchain = true;
    }

    pub fn add_additional_image_view(&mut self, key: usize, format: Format, usage: ImageUsage) {
        let final_view_image = self.final_views[0].image();

        let image = ImageView::new_default(
            Image::new(
                self.memory_allocator.clone(),
                ImageCreateInfo {
                    image_type: ImageType::Dim2d,
                    format,
                    extent: final_view_image.extent(),
                    usage,
                    ..Default::default()
                },
                AllocationCreateInfo::default(),
            )
            .unwrap(),
        )
        .unwrap();
        self.additional_image_views.insert(key, image);
    }

    pub fn get_additional_image_view(&mut self, key: usize) -> Arc<ImageView> {
        self.additional_image_views.get(&key).unwrap().clone()
    }

    pub fn remove_additional_image_view(&mut self, key: usize) {
        self.additional_image_views.remove(&key);
    }

    pub fn acquire(&mut self) -> Result<Box<dyn GpuFuture>, VulkanError> {
        if self.recreate_swapchain {
            self.recreate_swapchain_and_views();
        }

        let (image_index, suboptimal, acquire_future) =
            match swapchain::acquire_next_image(self.swapchain.clone(), None)
                .map_err(Validated::unwrap)
            {
                Ok(r) => r,
                Err(VulkanError::OutOfDate) => {
                    self.recreate_swapchain = true;
                    return Err(VulkanError::OutOfDate);
                }
                Err(e) => panic!("failed to acquire next image: {e}"),
            };
        if suboptimal {
            self.recreate_swapchain = true;
        }
        self.image_index = image_index;

        let future = self.previous_frame_end.take().unwrap().join(acquire_future);

        Ok(future.boxed())
    }

    pub fn present(&mut self, after_future: Box<dyn GpuFuture>, wait_future: bool) {
        let future = after_future
            .then_swapchain_present(
                self.gfx_queue.clone(),
                SwapchainPresentInfo::swapchain_image_index(
                    self.swapchain.clone(),
                    self.image_index,
                ),
            )
            .then_signal_fence_and_flush();
        match future.map_err(Validated::unwrap) {
            Ok(mut future) => {
                if wait_future {
                    match future.wait(None) {
                        Ok(x) => x,
                        Err(e) => println!("{e}"),
                    }
                    // wait allows you to organize resource waiting yourself.
                } else {
                    future.cleanup_finished();
                }

                self.previous_frame_end = Some(future.boxed());
            }
            Err(VulkanError::OutOfDate) => {
                self.recreate_swapchain = true;
                self.previous_frame_end = Some(sync::now(self.gfx_queue.device().clone()).boxed());
            }
            Err(e) => {
                println!("failed to flush future: {e}");
                self.previous_frame_end = Some(sync::now(self.gfx_queue.device().clone()).boxed());
            }
        }
    }

    fn recreate_swapchain_and_views(&mut self) {
        let image_extent: [u32; 2] = self.window().inner_size().into();

        if image_extent.contains(&0) {
            return;
        }

        let (new_swapchain, new_images) = self
            .swapchain
            .recreate(SwapchainCreateInfo {
                image_extent,
                // Use present mode from current state
                present_mode: self.present_mode,
                ..self.swapchain.create_info()
            })
            .expect("failed to recreate swapchain");

        self.swapchain = new_swapchain;
        let new_images = new_images
            .into_iter()
            .map(|image| ImageView::new_default(image).unwrap())
            .collect::<Vec<_>>();
        self.final_views = new_images;
        // Resize images that follow swapchain size
        let resizable_views = self
            .additional_image_views
            .iter()
            .map(|c| *c.0)
            .collect::<Vec<usize>>();
        for i in resizable_views {
            let format = self.get_additional_image_view(i).format();
            let usage = self.get_additional_image_view(i).usage();
            self.remove_additional_image_view(i);
            self.add_additional_image_view(i, format, usage);
        }
        self.recreate_swapchain = false;
    }
}
