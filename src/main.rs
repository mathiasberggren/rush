use rush::swapchain::{select_physical_device, get_render_pass, get_framebuffers};
use rush::pipeline::{get_pipeline, get_command_buffers, Vertex};
use vulkano::VulkanLibrary;
use vulkano::buffer::{CpuAccessibleBuffer, BufferUsage};
use vulkano::pipeline::graphics::viewport::Viewport;
use vulkano::{instance::Instance, instance::InstanceCreateInfo};
use vulkano_win::VkSurfaceBuild;
use vulkano::swapchain::SwapchainCreationError;
use winit::event_loop::EventLoop;
use winit::window::WindowBuilder;

mod vs {
    vulkano_shaders::shader! {
        ty: "vertex",
        src: "
            #version 450
            layout(location = 0) in vec2 position;
            void main() {
                gl_Position = vec4(position, 0.0, 1.0);
            }
        "
    }
}

mod fs {
    vulkano_shaders::shader! {
        ty: "fragment",
        src: "
            #version 450
            layout(location = 0) out vec4 f_color;
            void main() {
                f_color = vec4(1.0, 0.0, 0.0, 1.0);
            }
        "
    }
}


#[tokio::main]
async fn main() -> Result<(), reqwest::Error> {
    let library = VulkanLibrary::new().expect("no local Vulkan library/DLL");
    let required_extensions = vulkano_win::required_extensions(&library);
    let instance = Instance::new(
        library,
        InstanceCreateInfo {
            enumerate_portability: true,
            enabled_extensions: required_extensions,
            ..Default::default()
        }
    ).expect("failed to create instance");


    for physical_device in instance.enumerate_physical_devices().unwrap() {
        println!("Available device: {}", physical_device.properties().device_name);
    };

    let event_loop = EventLoop::new();  


    // Had to downgrade winit to 0.27.5 to get this to work
    let surface = WindowBuilder::new()
        .build_vk_surface(&event_loop, instance.clone())
        .unwrap();

    use winit::event::{Event, WindowEvent};
    use winit::event_loop::ControlFlow;
        

    use vulkano::device::DeviceExtensions;

    let device_extensions = DeviceExtensions {
        khr_swapchain: true,
        ..DeviceExtensions::empty()
    };

    // make sure we select the best device possible for rendering, prefereably a GPU
    let (physical_device, queue_family_index) = select_physical_device(&instance, &surface, &device_extensions);

    use vulkano::device::{Device, DeviceCreateInfo, QueueCreateInfo};

    // Create a logical device to support the swapchain
    let (device, mut queues) = Device::new(
        physical_device.clone(),
        DeviceCreateInfo {
            queue_create_infos: vec![QueueCreateInfo {
                queue_family_index,
                ..Default::default()
            }],
            enabled_extensions: device_extensions,
            ..Default::default()
        },
    )
    .expect("failed to create device");
    
    let queue = queues.next().unwrap();

    // Settings for the swapchain should be based on the settings of the surface
    let caps = physical_device
        .surface_capabilities(&surface, Default::default())
        .expect("failed to get surface capabilities");

    let dimensions = surface.window().inner_size();
    let composite_alpha = caps.supported_composite_alpha.iter().next().unwrap();
    let image_format = Some(
        physical_device
            .surface_formats(&surface, Default::default())
            .unwrap()[0]
            .0,
    );


    use vulkano::image::ImageUsage;
    use vulkano::swapchain::{Swapchain, SwapchainCreateInfo};

    // To ensure that only complete images are shown, Vulkan uses what is called a swapchain.
    // Basically we draw everything that is going to be rendered on a separate screen before displaying it.
    let  (mut swapchain, images) = Swapchain::new(
        device.clone(),
        surface.clone(),
        SwapchainCreateInfo {
            min_image_count: caps.min_image_count + 1, // How many buffers to use in the swapchain
            image_format,
            image_extent: dimensions.into(),
            image_usage: ImageUsage {
                color_attachment: true,  // What the images are going to be used for
                ..Default::default()
            },
            composite_alpha,
            ..Default::default()
        },
    ).unwrap();


    let vertex1 = Vertex {
        position: [-0.5, -0.5],
    };
    let vertex2 = Vertex {
        position: [0.0, 0.5],
    };
    let vertex3 = Vertex {
        position: [0.5, -0.25],
    };

    let vertex_buffer = CpuAccessibleBuffer::from_iter(
        device.clone(),
        BufferUsage {
            vertex_buffer: true,
            ..Default::default()
        },
        false,
        vec![vertex1, vertex2, vertex3].into_iter(),
    )
    .unwrap();

    let vs = vs::load(device.clone()).expect("failed to create shader module");
    let fs = fs::load(device.clone()).expect("failed to create shader module");

    let render_pass = get_render_pass(device.clone(), &swapchain);
    let framebuffers = get_framebuffers(&images, &render_pass);

    let mut viewport = Viewport {
        origin: [0.0, 0.0],
        dimensions: surface.window().inner_size().into(),
        depth_range: 0.0..1.0,
    };

    let pipeline = get_pipeline(
        device.clone(),
        vs.clone(),
        fs.clone(),
        render_pass.clone(),
        viewport.clone(),
    );

    let mut command_buffers =
    get_command_buffers(&device, &queue, &pipeline, &framebuffers, &vertex_buffer);


    // main()

    // Create infinity loop
    let mut window_resized = false;
    let mut recreate_swapchain = false;
    
    event_loop.run(move |event, _, control_flow| match event {
        Event::WindowEvent {
            event: WindowEvent::CloseRequested,
            ..
        } => {
            *control_flow = ControlFlow::Exit;
        }
        Event::WindowEvent {
            event: WindowEvent::Resized(_),
            ..
        } => {
            window_resized = true;
        }
        Event::MainEventsCleared => {}
        Event::RedrawEventsCleared => {
            if window_resized || recreate_swapchain {
                recreate_swapchain = false;
            
                let new_dimensions = surface.window().inner_size();
            
                let (new_swapchain, new_images) = match swapchain.recreate(SwapchainCreateInfo {
                    image_extent: new_dimensions.into(),
                    ..swapchain.create_info()
                }) {
                    Ok(r) => r,
                    Err(SwapchainCreationError::ImageExtentNotSupported { .. }) => return,
                    Err(e) => panic!("Failed to recreate swapchain: {:?}", e),
                };
                swapchain = new_swapchain;
                let new_framebuffers = get_framebuffers(&new_images, &render_pass);
            
                if window_resized {
                    window_resized = false;
            
                    viewport.dimensions = new_dimensions.into();
                    let new_pipeline = get_pipeline(
                        device.clone(),
                        vs.clone(),
                        fs.clone(),
                        render_pass.clone(),
                        viewport.clone(),
                    );
                    command_buffers = get_command_buffers(
                        &device,
                        &queue,
                        &new_pipeline,
                        &new_framebuffers,
                        &vertex_buffer,
                    );
                }
            }
        }
        _ => (),
    });


}


