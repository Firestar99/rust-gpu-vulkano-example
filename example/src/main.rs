// Copyright (c) 2016 The vulkano developers
// Licensed under the Apache License, Version 2.0
// <LICENSE-APACHE or
// https://www.apache.org/licenses/LICENSE-2.0> or the MIT
// license <LICENSE-MIT or https://opensource.org/licenses/MIT>,
// at your option. All files in the project carrying such
// notice may not be copied, modified, or distributed except
// according to those terms.

use vulkano::{
    buffer::{Buffer, BufferCreateInfo, BufferUsage, Subbuffer},
    command_buffer::{
        allocator::StandardCommandBufferAllocator, AutoCommandBufferBuilder, CommandBufferUsage,
        CopyBufferToImageInfo, PrimaryCommandBufferAbstract, RenderPassBeginInfo,
    },
    descriptor_set::{
        allocator::StandardDescriptorSetAllocator, DescriptorSet, WriteDescriptorSet,
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

pub mod shaders;
pub use shaders::fs;
pub use shaders::vs;

mod vulkano_example {
    use vulkano::{buffer::BufferContents, pipeline::graphics::vertex_input};

    #[derive(BufferContents, vertex_input::Vertex)]
    #[repr(C)]
    pub struct Vertex {
        #[format(R32G32_SFLOAT)]
        pub position: [f32; 2],
    }
}

struct App {
    vulkano_instance: std::sync::Arc<Instance>,
    device: std::sync::Arc<Device>,
    queue: std::sync::Arc<Queue>,
    command_buffer_allocator: std::sync::Arc<StandardCommandBufferAllocator>,
    descriptor_set_allocator: std::sync::Arc<StandardDescriptorSetAllocator>,
    memory_allocator: std::sync::Arc<StandardMemoryAllocator>,
    sampler: std::sync::Arc<Sampler>,
    vertex_buffer: Subbuffer<[vulkano_example::Vertex]>,
    texture_buffer: Subbuffer<[u8]>,
    render_ctx: Option<RenderContext>,
}

struct RenderContext {
    window: std::sync::Arc<Window>,
    swapchain: std::sync::Arc<Swapchain>,
    render_pass: std::sync::Arc<RenderPass>,
    framebuffers: Vec<std::sync::Arc<Framebuffer>>,
    viewport: Viewport,
    pipeline: std::sync::Arc<GraphicsPipeline>,
    descriptor_set: std::sync::Arc<DescriptorSet>,
    recreate_swapchain: bool,
    previous_frame_end: Option<Box<dyn GpuFuture>>,
}

impl App {
    fn new(event_loop: &EventLoop<()>) -> Self {
        let library = VulkanLibrary::new().unwrap();
        let required_extensions = Surface::required_extensions(&event_loop).unwrap();
        let vulkano_instance = Instance::new(
            library,
            InstanceCreateInfo {
                flags: InstanceCreateFlags::ENUMERATE_PORTABILITY,
                enabled_extensions: required_extensions,
                ..Default::default()
            },
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
        let (physical_device, queue_family_index) = vulkano_instance
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
                            && p.presentation_support(i as u32, event_loop)
                                .unwrap_or(false)
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
        let queue = queues.next().unwrap();

        let memory_allocator =
            std::sync::Arc::new(StandardMemoryAllocator::new_default(device.clone()));
        let command_buffer_allocator = std::sync::Arc::new(StandardCommandBufferAllocator::new(
            device.clone(),
            Default::default(),
        ));
        let descriptor_set_allocator = std::sync::Arc::new(StandardDescriptorSetAllocator::new(
            device.clone(),
            Default::default(),
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
        let vertex_buffer = Buffer::from_iter(
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
        .unwrap();

        let sampler = Sampler::new(
            device.clone(),
            SamplerCreateInfo {
                mag_filter: Filter::Linear,
                min_filter: Filter::Linear,
                address_mode: [SamplerAddressMode::Repeat; 3],
                ..Default::default()
            },
        )
        .unwrap();

        let texture_buffer = {
            let png_bytes = include_bytes!("image_img.png").as_slice();
            let decoder = png::Decoder::new(std::io::Cursor::new(png_bytes));

            // let decoder = png::Decoder::new(BufReader::new(
            //     File::open("example/src/image_img.png").unwrap(),
            // ));
            let reader = decoder.read_info().unwrap();
            let info = reader.info();

            Buffer::new_slice(
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
            .unwrap()
        };

        App {
            vulkano_instance,
            device,
            queue,
            command_buffer_allocator,
            descriptor_set_allocator,
            memory_allocator,
            sampler,
            vertex_buffer,
            texture_buffer,
            render_ctx: None,
        }
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        let window = std::sync::Arc::new(
            event_loop
                .create_window(Window::default_attributes())
                .unwrap(),
        );
        let surface = Surface::from_window(self.vulkano_instance.clone(), window.clone()).unwrap();

        let (swapchain, images) = {
            let surface_capabilities = self
                .device
                .physical_device()
                .surface_capabilities(&surface, Default::default())
                .unwrap();
            let image_format = self
                .device
                .clone()
                .physical_device()
                .surface_formats(&surface, Default::default())
                .unwrap()[0]
                .0;

            Swapchain::new(
                self.device.clone(),
                surface,
                SwapchainCreateInfo {
                    min_image_count: surface_capabilities.min_image_count.max(2),
                    image_format,
                    image_extent: window.clone().inner_size().into(),
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

        let render_pass = vulkano::single_pass_renderpass!(
            self.device.clone(),
            attachments: {
                color: {
                    format: swapchain.image_format(),
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
        .unwrap();

        let mut viewport = Viewport {
            offset: [0.0, 0.0],
            extent: [0.0, 0.0],
            depth_range: 0.0..=1.0,
        };
        let framebuffers = window_size_dependent_setup(&images, render_pass.clone(), &mut viewport);

        let mut uploads = AutoCommandBufferBuilder::primary(
            self.command_buffer_allocator.clone(),
            self.queue.queue_family_index(),
            CommandBufferUsage::OneTimeSubmit,
        )
        .unwrap();

        let texture = {
            let png_bytes = include_bytes!("image_img.png").as_slice();
            let decoder = png::Decoder::new(std::io::Cursor::new(png_bytes));

            // let decoder = png::Decoder::new(BufReader::new(
            //     File::open("example/src/image_img.png").unwrap(),
            // ));
            let mut reader = decoder.read_info().unwrap();
            let info = reader.info();
            let extent = [info.width, info.height, 1];

            reader
                .next_frame(&mut self.texture_buffer.write().unwrap())
                .unwrap();

            let image = Image::new(
                self.memory_allocator.clone(),
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

            uploads
                .copy_buffer_to_image(CopyBufferToImageInfo::buffer_image(
                    self.texture_buffer.clone(),
                    image.clone(),
                ))
                .unwrap();

            ImageView::new_default(image).unwrap()
        };

        let pipeline = {
            let vs = vs::load(self.device.clone())
                .unwrap()
                .single_entry_point()
                .unwrap();
            let fs = fs::load(self.device.clone())
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
                self.device.clone(),
                PipelineDescriptorSetLayoutCreateInfo::from_stages(&stages)
                    .into_pipeline_layout_create_info(self.device.clone())
                    .unwrap(),
            )
            .unwrap();
            let subpass = Subpass::from(render_pass.clone(), 0).unwrap();

            GraphicsPipeline::new(
                self.device.clone(),
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
        }
        .unwrap();

        let layout = pipeline.layout().set_layouts().first().unwrap().clone();

        let descriptor_set = DescriptorSet::new(
            self.descriptor_set_allocator.clone(),
            layout.clone(),
            [
                WriteDescriptorSet::sampler(0, self.sampler.clone()),
                WriteDescriptorSet::image_view(1, texture.clone()),
            ],
            [],
        )
        .unwrap();

        let previous_frame_end = Some(
            uploads
                .build()
                .unwrap()
                .execute(self.queue.clone())
                .unwrap()
                .boxed(),
        );

        self.render_ctx = Some(RenderContext {
            window,
            swapchain,
            render_pass,
            viewport,
            framebuffers,
            pipeline,
            descriptor_set,
            recreate_swapchain: false,
            previous_frame_end,
        });
    }

    fn window_event(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
        _window_id: winit::window::WindowId,
        event: WindowEvent,
    ) {
        let render_ctx = self.render_ctx.as_mut().unwrap();

        match event {
            WindowEvent::CloseRequested => {
                event_loop.exit();
            }
            WindowEvent::Resized(..) => {
                render_ctx.recreate_swapchain = true;
            }
            WindowEvent::RedrawRequested => {
                let image_extent: [u32; 2] = render_ctx.window.clone().inner_size().into();

                if image_extent.contains(&0) {
                    return;
                }

                render_ctx
                    .previous_frame_end
                    .as_mut()
                    .unwrap()
                    .cleanup_finished();

                if render_ctx.recreate_swapchain {
                    let (new_swapchain, new_images) = render_ctx
                        .swapchain
                        .recreate(SwapchainCreateInfo {
                            image_extent,
                            ..render_ctx.swapchain.create_info()
                        })
                        .expect("failed to recreate swapchain");

                    render_ctx.swapchain = new_swapchain;
                    render_ctx.framebuffers = window_size_dependent_setup(
                        &new_images,
                        render_ctx.render_pass.clone(),
                        &mut render_ctx.viewport,
                    );
                    render_ctx.recreate_swapchain = false;
                }

                let (image_index, suboptimal, acquire_future) =
                    match acquire_next_image(render_ctx.swapchain.clone(), None)
                        .map_err(Validated::unwrap)
                    {
                        Ok(r) => r,
                        Err(VulkanError::OutOfDate) => {
                            render_ctx.recreate_swapchain = true;
                            return;
                        }
                        Err(e) => panic!("failed to acquire next image: {e}"),
                    };

                if suboptimal {
                    render_ctx.recreate_swapchain = true;
                }

                let mut builder = AutoCommandBufferBuilder::primary(
                    self.command_buffer_allocator.clone(),
                    self.queue.queue_family_index(),
                    CommandBufferUsage::OneTimeSubmit,
                )
                .unwrap();
                unsafe {
                    builder
                        .begin_render_pass(
                            RenderPassBeginInfo {
                                clear_values: vec![Some([0.0, 0.0, 1.0, 1.0].into())],
                                ..RenderPassBeginInfo::framebuffer(
                                    render_ctx.framebuffers[image_index as usize].clone(),
                                )
                            },
                            Default::default(),
                        )
                        .unwrap()
                        .set_viewport(0, [render_ctx.viewport.clone()].into_iter().collect())
                        .unwrap()
                        .bind_pipeline_graphics(render_ctx.pipeline.clone())
                        .unwrap()
                        .bind_descriptor_sets(
                            PipelineBindPoint::Graphics,
                            render_ctx.pipeline.layout().clone(),
                            0,
                            render_ctx.descriptor_set.clone(),
                        )
                        .unwrap()
                        .bind_vertex_buffers(0, self.vertex_buffer.clone())
                        .unwrap()
                        .draw(self.vertex_buffer.len() as u32, 1, 0, 0)
                        .unwrap()
                        .end_render_pass(Default::default())
                        .unwrap();
                }
                let command_buffer = builder.build().unwrap();

                let future = render_ctx
                    .previous_frame_end
                    .take()
                    .unwrap()
                    .join(acquire_future)
                    .then_execute(self.queue.clone(), command_buffer)
                    .unwrap()
                    .then_swapchain_present(
                        self.queue.clone(),
                        SwapchainPresentInfo::swapchain_image_index(
                            render_ctx.swapchain.clone(),
                            image_index,
                        ),
                    )
                    .then_signal_fence_and_flush();

                match future.map_err(Validated::unwrap) {
                    Ok(future) => {
                        render_ctx.previous_frame_end = Some(future.boxed());
                    }
                    Err(VulkanError::OutOfDate) => {
                        render_ctx.recreate_swapchain = true;
                        render_ctx.previous_frame_end =
                            Some(sync::now(self.device.clone()).boxed());
                    }
                    Err(e) => {
                        println!("failed to flush future: {e}");
                        render_ctx.previous_frame_end =
                            Some(sync::now(self.device.clone()).boxed());
                    }
                }
                // Since the image does not change, should we redraw it each frame ?
                // self.window.as_ref().unwrap().request_redraw();
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
    let mut app = App::new(&event_loop);
    event_loop.run_app(&mut app).unwrap();
}

/// This function is called once during initialization, then again whenever the window is resized.
fn window_size_dependent_setup(
    images: &[std::sync::Arc<Image>],
    render_pass: std::sync::Arc<RenderPass>,
    viewport: &mut Viewport,
) -> Vec<std::sync::Arc<Framebuffer>> {
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
