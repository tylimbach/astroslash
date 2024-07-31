mod graphics;

use graphics::{Graphics, GraphicsBuilder, MaybeGraphics};
#[allow(unused_imports)]
use wasm_bindgen::{prelude::wasm_bindgen, throw_str, JsCast, UnwrapThrowExt};

use winit::{
    application::ApplicationHandler,
    dpi::PhysicalSize,
    event::WindowEvent,
    event_loop::{ActiveEventLoop, EventLoop},
    window::WindowId,
};

struct Application {
    graphics: MaybeGraphics,
    clear_color: wgpu::Color,
}

impl Application {
    fn new(event_loop: &EventLoop<Graphics>) -> Self {
        Self {
            graphics: MaybeGraphics::Builder(GraphicsBuilder::new(event_loop.create_proxy())),
            clear_color: wgpu::Color::BLACK,
        }
    }

    fn draw(&mut self) {
        let MaybeGraphics::Graphics(gfx) = &mut self.graphics else {
            // draw call rejected because graphics doesn't exist yet
            return;
        };

        let frame = gfx.surface.get_current_texture().unwrap_throw();
        let view = frame.texture.create_view(&Default::default());
        let mut encoder = gfx.device.create_command_encoder(&Default::default());

        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(self.clear_color),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                ..Default::default()
            });
            render_pass.set_pipeline(&gfx.render_pipeline);
            render_pass.draw(0..3, 0..1);
        }

        let command_buffer = encoder.finish();
        gfx.queue.submit([command_buffer]);
        frame.present();
    }

    fn resized(&mut self, size: PhysicalSize<u32>) {
        let MaybeGraphics::Graphics(gfx) = &mut self.graphics else {
            return;
        };
        gfx.surface_config.width = size.width;
        gfx.surface_config.height = size.height;
        gfx.surface.configure(&gfx.device, &gfx.surface_config);
    }
}

impl ApplicationHandler<Graphics> for Application {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if let MaybeGraphics::Builder(builder) = &mut self.graphics {
            builder.build_and_send(event_loop);
        }
    }

    fn user_event(&mut self, _event_loop: &ActiveEventLoop, graphics: Graphics) {
        self.graphics = MaybeGraphics::Graphics(graphics);
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: WindowId,
        event: WindowEvent,
    ) {
        match event {
            WindowEvent::Resized(size) => self.resized(size),
            WindowEvent::RedrawRequested => self.draw(),
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::CursorMoved {
                device_id,
                position,
            } => {
                self.clear_color = wgpu::Color {
                    r: (position.x % 256.0) / 256.0,
                    g: (position.y % 256.0) / 256.0,
                    b: ((position.x - position.y) % 256.0) / 256.0,
                    a: 1.0,
                };
                self.draw();
            }
            WindowEvent::KeyboardInput {
                device_id,
                event,
                is_synthetic,
            } => {}
            _ => (),
        }
    }
}

pub fn run() {
    let event_loop = EventLoop::with_user_event().build().unwrap_throw();
    let mut app = Application::new(&event_loop);
    event_loop.run_app(&mut app).unwrap_throw();
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn run_web() {
    let window = web_sys::window().unwrap_throw();
    let document = window.document().unwrap_throw();

    let canvas = document.create_element("canvas").unwrap_throw();
    canvas.set_id(graphics::CANVAS_ID);
    canvas.set_attribute("width", "500").unwrap_throw();
    canvas.set_attribute("height", "500").unwrap_throw();

    let body = document
        .get_elements_by_tag_name("body")
        .item(0)
        .unwrap_throw();
    body.append_with_node_1(canvas.unchecked_ref())
        .unwrap_throw();

    run();
}
