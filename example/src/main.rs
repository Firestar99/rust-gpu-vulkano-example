// Copyright (c) 2016 The vulkano developers
// Licensed under the Apache License, Version 2.0
// <LICENSE-APACHE or
// https://www.apache.org/licenses/LICENSE-2.0> or the MIT
// license <LICENSE-MIT or https://opensource.org/licenses/MIT>,
// at your option. All files in the project carrying such
// notice may not be copied, modified, or distributed except
// according to those terms.

use std::sync::Arc;
use vulkano::device::Features;
use vulkano::{
    buffer::{Buffer, BufferCreateInfo, BufferUsage, Subbuffer},
    command_buffer::{
        allocator::StandardCommandBufferAllocator, AutoCommandBufferBuilder, CommandBufferUsage,
        PrimaryCommandBufferAbstract, RenderPassBeginInfo,
    },
    descriptor_set::{
        allocator::StandardDescriptorSetAllocator, layout::DescriptorSetLayout, DescriptorSet,
        WriteDescriptorSet,
    },
    device::{
        physical::PhysicalDeviceType, Device, DeviceCreateInfo, DeviceExtensions, DeviceFeatures,
        Queue, QueueCreateInfo, QueueFlags,
    },
    format::Format,
    image::{
        sampler::{Filter, Sampler, SamplerAddressMode, SamplerCreateInfo},
        view::ImageView,
        Image, ImageCreateInfo, ImageType, ImageUsage,
    },
    instance::{Instance, InstanceCreateFlags, InstanceCreateInfo},
    memory::allocator::{AllocationCreateInfo, MemoryTypeFilter, StandardMemoryAllocator},
    pipeline::{
        graphics::{
            color_blend::{AttachmentBlend, ColorBlendAttachmentState, ColorBlendState},
            input_assembly::{InputAssemblyState, PrimitiveTopology},
            multisample::MultisampleState,
            rasterization::RasterizationState,
            vertex_input::{Vertex, VertexDefinition},
            viewport::{Viewport, ViewportState},
            GraphicsPipelineCreateInfo,
        },
        layout::PipelineDescriptorSetLayoutCreateInfo,
        DynamicState, GraphicsPipeline, Pipeline, PipelineBindPoint, PipelineLayout,
        PipelineShaderStageCreateInfo,
    },
    render_pass::{Framebuffer, FramebufferCreateInfo, RenderPass, Subpass},
    swapchain::{
        acquire_next_image, Surface, Swapchain, SwapchainCreateInfo, SwapchainPresentInfo,
    },
    sync::{self, GpuFuture},
    DeviceSize, Validated, VulkanError, VulkanLibrary,
};
use winit::{
    application::ApplicationHandler, event::WindowEvent, event_loop::EventLoop, keyboard::Key,
    window::Window,
};

mod vulkano_example {
    use vulkano::{buffer::BufferContents, pipeline::graphics::vertex_input};

    #[derive(BufferContents, vertex_input::Vertex)]
    #[repr(C)]
    pub struct Vertex {
        #[format(R32G32_SFLOAT)]
        pub position: [f32; 2],
    }
}

#[derive(Default)]
struct App {
    window: Option<Arc<Window>>,
    recreate_swapchain: bool,
    vulkano_instance: Option<Arc<Instance>>,
    queue: Option<Arc<Queue>>,
    swapchain: Option<Arc<Swapchain>>,
    device: Option<Arc<Device>>,
    framebuffers: Vec<Arc<Framebuffer>>,
    render_pass: Option<Arc<RenderPass>>,
    viewport: Viewport,
    pipeline: Option<Arc<GraphicsPipeline>>,
    vertex_buffer: Option<Subbuffer<[vulkano_example::Vertex]>>,
    command_buffer_allocator: Option<Arc<StandardCommandBufferAllocator>>,
    descriptor_set_allocator: Option<Arc<StandardDescriptorSetAllocator>>,
    layout: Option<Arc<DescriptorSetLayout>>,
    descriptor_set: Option<Arc<DescriptorSet>>,
    sampler: Option<Arc<Sampler>>,
    texture: Option<Arc<ImageView>>,
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        println!("Resumed");
        self.window = Some(std::sync::Arc::new(
            event_loop
                .create_window(Window::default_attributes())
                .unwrap(),
        ));

        let library = VulkanLibrary::new().unwrap();
        let required_extensions = Surface::required_extensions(&event_loop).unwrap();
        self.vulkano_instance = Some(
            Instance::new(
                library,
                InstanceCreateInfo {
                    flags: InstanceCreateFlags::ENUMERATE_PORTABILITY,
                    enabled_extensions: required_extensions,
                    ..Default::default()
                },
            )
            .unwrap(),
        );

        let surface = Surface::from_window(
            self.vulkano_instance.as_ref().unwrap().clone(),
            self.window.clone().unwrap(),
        )
        .unwrap();
        let device_extensions = DeviceExtensions {
            khr_swapchain: true,
            khr_vulkan_memory_model: true,
            ..DeviceExtensions::empty()
        };
        let features = DeviceFeatures {
            vulkan_memory_model: true,
            ..DeviceFeatures::empty()
        };
        let (physical_device, queue_family_index) = self
            .vulkano_instance
            .as_ref()
            .unwrap()
            .enumerate_physical_devices()
            .unwrap()
            .filter(|p| p.supported_extensions().contains(&device_extensions))
            .filter(|p| p.supported_features().contains(&features))
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
            .min_by_key(|(p, _)| match p.properties().device_type {
                PhysicalDeviceType::DiscreteGpu => 0,
                PhysicalDeviceType::IntegratedGpu => 1,
                PhysicalDeviceType::VirtualGpu => 2,
                PhysicalDeviceType::Cpu => 3,
                PhysicalDeviceType::Other => 4,
                _ => 5,
            })
            .unwrap();

        println!(
            "Using device: {} (type: {:?})",
            physical_device.properties().device_name,
            physical_device.properties().device_type,
        );

        let (device, mut queues) = Device::new(
            physical_device,
            DeviceCreateInfo {
                enabled_extensions: device_extensions,
                enabled_features: features,
                queue_create_infos: vec![QueueCreateInfo {
                    queue_family_index,
                    ..Default::default()
                }],
                ..Default::default()
            },
        )
        .unwrap();
        self.device = Some(device);
        self.queue = queues.next();

        let (swapchain, images) = {
            let surface_capabilities = self
                .device
                .as_ref()
                .unwrap()
                .physical_device()
                .surface_capabilities(&surface, Default::default())
                .unwrap();
            let image_format = self
                .device
                .as_ref()
                .unwrap()
                .physical_device()
                .surface_formats(&surface, Default::default())
                .unwrap()[0]
                .0;

            Swapchain::new(
                self.device.as_ref().unwrap().clone(),
                surface,
                SwapchainCreateInfo {
                    min_image_count: surface_capabilities.min_image_count.max(2),
                    image_format,
                    image_extent: self.window.clone().unwrap().inner_size().clone().into(),
                    image_usage: ImageUsage::COLOR_ATTACHMENT,
                    composite_alpha: surface_capabilities
                        .supported_composite_alpha
                        .into_iter()
                        .next()
                        .unwrap(),
                    ..Default::default()
                },
            )
            .unwrap()
        };
        self.swapchain = Some(swapchain);
        let memory_allocator = Arc::new(StandardMemoryAllocator::new_default(
            self.device.as_ref().unwrap().clone(),
        ));

        let vertices = [
            vulkano_example::Vertex {
                position: [-0.5, -0.5],
            },
            vulkano_example::Vertex {
                position: [-0.5, 0.5],
            },
            vulkano_example::Vertex {
                position: [0.5, -0.5],
            },
            vulkano_example::Vertex {
                position: [0.5, 0.5],
            },
        ];
        self.vertex_buffer = Buffer::from_iter(
            memory_allocator.clone(),
            BufferCreateInfo {
                usage: BufferUsage::VERTEX_BUFFER,
                ..Default::default()
            },
            AllocationCreateInfo {
                memory_type_filter: MemoryTypeFilter::PREFER_DEVICE
                    | MemoryTypeFilter::HOST_SEQUENTIAL_WRITE,
                ..Default::default()
            },
            vertices,
        )
        .ok();

        self.render_pass = vulkano::single_pass_renderpass!(
            self.device.as_ref().unwrap().clone(),
            attachments: {
                color: {
                    format: self.swapchain.as_ref().unwrap().image_format(),
                    samples: 1,
                    load_op: Clear,
                    store_op: Store,
                },
            },
            pass: {
                color: [color],
                depth_stencil: {},
            },
        )
        .ok();

        self.descriptor_set_allocator =
            Some(std::sync::Arc::new(StandardDescriptorSetAllocator::new(
                self.device.as_ref().unwrap().clone(),
                Default::default(),
            )));
        self.command_buffer_allocator =
            Some(std::sync::Arc::new(StandardCommandBufferAllocator::new(
                self.device.as_ref().unwrap().clone(),
                Default::default(),
            )));

        self.texture = {
            let png_bytes = include_bytes!("image_img.png").as_slice();
            let decoder = png::Decoder::new(png_bytes);
            let mut reader = decoder.read_info().unwrap();
            let info = reader.info();
            let extent = [info.width, info.height, 1];

            let upload_buffer = Buffer::new_slice(
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
                (info.width * info.height * 4) as DeviceSize,
            )
            .unwrap();

            reader
                .next_frame(&mut upload_buffer.write().unwrap())
                .unwrap();

            let image = Image::new(
                memory_allocator,
                ImageCreateInfo {
                    image_type: ImageType::Dim2d,
                    format: Format::R8G8B8A8_SRGB,
                    extent,
                    usage: ImageUsage::TRANSFER_DST | ImageUsage::SAMPLED,
                    ..Default::default()
                },
                AllocationCreateInfo::default(),
            )
            .unwrap();

            ImageView::new_default(image).ok()
        };

        self.sampler = Sampler::new(
            self.device.as_ref().unwrap().clone(),
            SamplerCreateInfo {
                mag_filter: Filter::Linear,
                min_filter: Filter::Linear,
                address_mode: [SamplerAddressMode::Repeat; 3],
                ..Default::default()
            },
        )
        .ok();

        self.pipeline = {
            let vs = vs::load(self.device.as_ref().unwrap().clone())
                .unwrap()
                .single_entry_point()
                .unwrap();
            let fs = fs::load(self.device.as_ref().unwrap().clone())
                .unwrap()
                .single_entry_point()
                .unwrap();
            let vertex_input_state = vulkano_example::Vertex::per_vertex()
                .definition(&vs)
                .unwrap();
            let stages = [
                PipelineShaderStageCreateInfo::new(vs),
                PipelineShaderStageCreateInfo::new(fs),
            ];
            let layout = PipelineLayout::new(
                self.device.as_ref().unwrap().clone(),
                PipelineDescriptorSetLayoutCreateInfo::from_stages(&stages)
                    .into_pipeline_layout_create_info(self.device.as_ref().unwrap().clone())
                    .unwrap(),
            )
            .unwrap();
            let subpass = Subpass::from(self.render_pass.as_ref().unwrap().clone(), 0).unwrap();

            GraphicsPipeline::new(
                self.device.as_ref().unwrap().clone(),
                None,
                GraphicsPipelineCreateInfo {
                    stages: stages.into_iter().collect(),
                    vertex_input_state: Some(vertex_input_state),
                    input_assembly_state: Some(InputAssemblyState {
                        topology: PrimitiveTopology::TriangleStrip,
                        ..Default::default()
                    }),
                    viewport_state: Some(ViewportState::default()),
                    rasterization_state: Some(RasterizationState::default()),
                    multisample_state: Some(MultisampleState::default()),
                    color_blend_state: Some(ColorBlendState::with_attachment_states(
                        subpass.num_color_attachments(),
                        ColorBlendAttachmentState {
                            blend: Some(AttachmentBlend::alpha()),
                            ..Default::default()
                        },
                    )),
                    dynamic_state: [DynamicState::Viewport].into_iter().collect(),
                    subpass: Some(subpass.into()),
                    ..GraphicsPipelineCreateInfo::layout(layout)
                },
            )
            .ok()
        };

        self.layout = Some(
            self.pipeline
                .as_ref()
                .unwrap()
                .layout()
                .set_layouts()
                .get(0)
                .unwrap()
                .clone(),
        );

        self.descriptor_set = DescriptorSet::new(
            self.descriptor_set_allocator.as_ref().unwrap().clone(),
            self.layout.as_ref().unwrap().clone(),
            [
                WriteDescriptorSet::sampler(0, self.sampler.as_ref().unwrap().clone()),
                WriteDescriptorSet::image_view(1, self.texture.as_ref().unwrap().clone()),
            ],
            [],
        )
        .ok();

        let mut viewport = Viewport {
            offset: [0.0, 0.0],
            extent: [0.0, 0.0],
            depth_range: 0.0..=1.0,
        };
        self.framebuffers = window_size_dependent_setup(
            &images,
            self.render_pass.as_ref().unwrap().clone(),
            &mut viewport,
        );

        self.recreate_swapchain = false;
    }

    fn window_event(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
        _window_id: winit::window::WindowId,
        event: WindowEvent,
    ) {
        match event {
            WindowEvent::CloseRequested => {
                event_loop.exit();
            }
            WindowEvent::Resized(..) => {
                self.recreate_swapchain = true;
            }
            WindowEvent::RedrawRequested => {
                println!("Redraw");
                let image_extent: [u32; 2] = self
                    .window
                    .clone()
                    .expect("Cannot retrieve a valid window")
                    .inner_size()
                    .into();

                if image_extent.contains(&0) {
                    return;
                }

                let uploads = AutoCommandBufferBuilder::primary(
                    std::sync::Arc::new(self.command_buffer_allocator.as_ref().unwrap().clone()),
                    self.queue.as_ref().unwrap().queue_family_index(),
                    CommandBufferUsage::OneTimeSubmit,
                )
                .unwrap();

                let mut previous_frame_end = Some(
                    uploads
                        .build()
                        .unwrap()
                        .execute(self.queue.as_ref().unwrap().clone())
                        .unwrap()
                        .boxed(),
                );

                previous_frame_end.as_mut().unwrap().cleanup_finished();

                if self.recreate_swapchain {
                    let (new_swapchain, new_images) = self
                        .swapchain
                        .as_ref()
                        .unwrap()
                        .recreate(SwapchainCreateInfo {
                            image_extent,
                            ..self.swapchain.as_ref().unwrap().create_info()
                        })
                        .expect("failed to recreate swapchain");

                    self.swapchain = Some(new_swapchain);
                    self.framebuffers = window_size_dependent_setup(
                        &new_images,
                        self.render_pass.as_ref().unwrap().clone(),
                        &mut self.viewport,
                    );
                    self.recreate_swapchain = false;
                }

                let (image_index, suboptimal, acquire_future) =
                    match acquire_next_image(self.swapchain.as_ref().unwrap().clone(), None)
                        .map_err(Validated::unwrap)
                    {
                        Ok(r) => r,
                        Err(VulkanError::OutOfDate) => {
                            self.recreate_swapchain = true;
                            return;
                        }
                        Err(e) => panic!("failed to acquire next image: {e}"),
                    };

                if suboptimal {
                    self.recreate_swapchain = true;
                }

                let mut builder = AutoCommandBufferBuilder::primary(
                    self.command_buffer_allocator.as_ref().unwrap().clone(),
                    self.queue.as_ref().unwrap().queue_family_index(),
                    CommandBufferUsage::OneTimeSubmit,
                )
                .unwrap();
                unsafe {
                    builder
                        .begin_render_pass(
                            RenderPassBeginInfo {
                                clear_values: vec![Some([0.0, 0.0, 1.0, 1.0].into())],
                                ..RenderPassBeginInfo::framebuffer(
                                    self.framebuffers[image_index as usize].clone(),
                                )
                            },
                            Default::default(),
                        )
                        .unwrap()
                        .set_viewport(0, [self.viewport.clone()].into_iter().collect())
                        .unwrap()
                        .bind_pipeline_graphics(self.pipeline.as_ref().unwrap().clone())
                        .unwrap()
                        .bind_descriptor_sets(
                            PipelineBindPoint::Graphics,
                            self.pipeline.as_ref().unwrap().layout().clone(),
                            0,
                            self.descriptor_set.as_ref().unwrap().clone(),
                        )
                        .unwrap()
                        .bind_vertex_buffers(0, self.vertex_buffer.as_ref().unwrap().clone())
                        .unwrap()
                        .draw(self.vertex_buffer.as_ref().unwrap().len() as u32, 1, 0, 0)
                        .unwrap()
                        .end_render_pass(Default::default())
                        .unwrap();
                }
                let command_buffer = builder.build().unwrap();

                let future = previous_frame_end
                    .take()
                    .unwrap()
                    .join(acquire_future)
                    .then_execute(self.queue.as_ref().unwrap().clone(), command_buffer)
                    .unwrap()
                    .then_swapchain_present(
                        self.queue.as_ref().unwrap().clone(),
                        SwapchainPresentInfo::swapchain_image_index(
                            self.swapchain.as_ref().unwrap().clone(),
                            image_index,
                        ),
                    )
                    .then_signal_fence_and_flush();

                match future.map_err(Validated::unwrap) {
                    Ok(future) => {
                        Some(future.boxed());
                    }
                    Err(VulkanError::OutOfDate) => {
                        self.recreate_swapchain = true;
                        Some(sync::now(self.device.as_ref().unwrap().clone()).boxed());
                    }
                    Err(e) => {
                        println!("failed to flush future: {e}");
                        Some(sync::now(self.device.as_ref().unwrap().clone()).boxed());
                    }
                }
                self.window.as_ref().unwrap().request_redraw();
            }
            WindowEvent::KeyboardInput { event, .. } => {
                if event.logical_key == Key::Named(winit::keyboard::NamedKey::Escape) {
                    event_loop.exit();
                }
            }
            _ => (),
        }
    }
}

fn main() {
    // The start of this example is exactly the same as `triangle`. You should read the `triangle`
    // example if you haven't done so yet.

    let event_loop = EventLoop::new().unwrap();
    let mut app = App::default();
    event_loop.run_app(&mut app).unwrap();
}

/// This function is called once during initialization, then again whenever the window is resized.
fn window_size_dependent_setup(
    images: &[Arc<Image>],
    render_pass: Arc<RenderPass>,
    viewport: &mut Viewport,
) -> Vec<Arc<Framebuffer>> {
    let extent = images[0].extent();
    viewport.extent = [extent[0] as f32, extent[1] as f32];

    images
        .iter()
        .map(|image| {
            let view = ImageView::new_default(image.clone()).unwrap();
            Framebuffer::new(
                render_pass.clone(),
                FramebufferCreateInfo {
                    attachments: vec![view],
                    ..Default::default()
                },
            )
            .unwrap()
        })
        .collect::<Vec<_>>()
}

pub mod shaders;

pub use shaders::vs;

pub use shaders::fs;
