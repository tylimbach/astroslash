use std::{borrow::Cow, future::Future};

#[allow(unused_imports)]
use wasm_bindgen::{prelude::wasm_bindgen, throw_str, JsCast, UnwrapThrowExt};

use wgpu::util::DeviceExt;
use wgpu::{
    Adapter, Buffer, Device, Instance, Queue, RenderPipeline, Surface, SurfaceConfiguration,
};
use winit::{
    event_loop::{ActiveEventLoop, EventLoopProxy},
    window::Window,
};

#[cfg(target_arch = "wasm32")]
type Rc<T> = std::rc::Rc<T>;

#[cfg(not(target_arch = "wasm32"))]
type Rc<T> = std::sync::Arc<T>;

#[cfg(target_arch = "wasm32")]
pub const CANVAS_ID: &str = "wgpu-canvas";

#[allow(dead_code)]
pub struct Graphics {
    pub window: Rc<Window>,
    pub instance: Instance,
    pub surface: Surface<'static>,
    pub surface_config: SurfaceConfiguration,
    pub adapter: Adapter,
    pub device: Device,
    pub queue: Queue,
    pub render_pipeline: RenderPipeline,
    pub vertex_buffer: Buffer,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Vertex {
    pub position: [f32; 3],
    pub color: [f32; 3],
}

impl Vertex {
    fn desc() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Vertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 0,
                    format: wgpu::VertexFormat::Float32x3,
                },
                wgpu::VertexAttribute {
                    offset: std::mem::size_of::<[f32; 3]>() as wgpu::BufferAddress,
                    shader_location: 1,
                    format: wgpu::VertexFormat::Float32x3,
                },
            ],
        }
    }
}

const VERTICES: &[Vertex] = &[
    Vertex {
        position: [0.0, 0.5, 0.0],
        color: [1.0, 0.0, 0.0],
    },
    Vertex {
        position: [-0.5, -0.5, 0.0],
        color: [0.0, 1.0, 0.0],
    },
    Vertex {
        position: [0.5, 0.5, 0.0],
        color: [1.0, 0.0, 1.0],
    },
];

pub fn create_graphics(event_loop: &ActiveEventLoop) -> impl Future<Output = Graphics> + 'static {
    #[allow(unused_mut)]
    let mut window_attrs = Window::default_attributes();

    #[cfg(target_arch = "wasm32")]
    {
        use web_sys::wasm_bindgen::JsCast;
        use winit::platform::web::WindowAttributesExtWebSys;

        let window = web_sys::window().unwrap_throw();
        let document = window.document().unwrap_throw();
        let canvas = document.get_element_by_id(CANVAS_ID).unwrap_throw();
        let html_canvas_element = canvas.unchecked_into();
        window_attrs = window_attrs.with_canvas(Some(html_canvas_element));
    }

    let window = Rc::new(event_loop.create_window(window_attrs).unwrap_throw());
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
        #[cfg(not(target_arch = "wasm32"))]
        backends: wgpu::Backends::PRIMARY,
        #[cfg(target_arch = "wasm32")]
        backends: wgpu::Backends::GL,
        ..Default::default()
    });

    let surface = instance
        .create_surface(window.clone())
        .unwrap_or_else(|e| throw_str(&format!("{e:#?}")));

    async move {
        // note: may need to enumerate adapters for some devices?
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                compatible_surface: Some(&surface),
                power_preference: wgpu::PowerPreference::None,
                force_fallback_adapter: false,
            })
            .await
            .unwrap_throw();

        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: None,
                    required_features: wgpu::Features::empty(),
                    required_limits: if cfg!(target_arch = "wasm32") {
                        wgpu::Limits::downlevel_webgl2_defaults()
                    } else {
                        wgpu::Limits::default()
                    },
                    memory_hints: wgpu::MemoryHints::MemoryUsage,
                },
                None,
            )
            .await
            .unwrap_throw();

        let size = window.inner_size();
        let surface_config = surface
            .get_default_config(&adapter, size.width, size.height)
            .unwrap_throw();

        #[cfg(not(target_arch = "wasm32"))]
        {
            surface.configure(&device, &surface_config);
        }

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Render Pipeline Layout"),
            bind_group_layouts: &[],
            push_constant_ranges: &[],
        });
        let swapchain_capabilities = surface.get_capabilities(&adapter);

        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Vertex Buffer"),
            contents: bytemuck::cast_slice(VERTICES),
            usage: wgpu::BufferUsages::VERTEX,
        });

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Shader"),
            source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(include_str!("shader.wgsl"))),
        });

        // let swapchain_format = swapchain_capabilities.formats[0];
        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Render Pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "vs_main",
                compilation_options: Default::default(),
                buffers: &[Vertex::desc()],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: "fs_main",
                compilation_options: Default::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format: surface_config.format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                //targets: &[Some(wapchain_format.into())],
            }),
            primitive: Default::default(),
            depth_stencil: None,
            multisample: Default::default(),
            multiview: None,
            cache: None,
        });

        Graphics {
            window,
            instance,
            surface,
            surface_config,
            adapter,
            device,
            queue,
            render_pipeline,
            vertex_buffer,
        }
    }
}

pub struct GraphicsBuilder {
    event_loop_proxy: Option<EventLoopProxy<Graphics>>,
}

impl GraphicsBuilder {
    pub fn new(event_loop_proxy: EventLoopProxy<Graphics>) -> Self {
        Self {
            event_loop_proxy: Some(event_loop_proxy),
        }
    }

    pub fn build_and_send(&mut self, event_loop: &ActiveEventLoop) {
        let Some(event_loop_proxy) = self.event_loop_proxy.take() else {
            // event_loop_proxy is already spent - we already constructed Graphics
            return;
        };

        #[cfg(target_arch = "wasm32")]
        {
            let gfx_fut = create_graphics(event_loop);
            wasm_bindgen_futures::spawn_local(async move {
                let gfx = gfx_fut.await;
                assert!(event_loop_proxy.send_event(gfx).is_ok());
            });
        }

        #[cfg(not(target_arch = "wasm32"))]
        {
            let gfx = pollster::block_on(create_graphics(event_loop));
            assert!(event_loop_proxy.send_event(gfx).is_ok());
        }
    }
}

pub enum MaybeGraphics {
    Builder(GraphicsBuilder),
    Graphics(Graphics),
}
